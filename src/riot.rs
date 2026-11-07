//! Riot sign-in helpers for the opt-in Valorant player-card fetch.
//!
//! The equipped player card is not stored on disk; the game asks Riot's
//! personalisation API for it. This module reproduces that call with the
//! account that is *currently* signed in to the Riot Client, and nothing else:
//!
//! 1. read the refresh token the client keeps in `RiotGamesPrivateSettings.yaml`
//! 2. exchange it for an access token (Riot rotates the refresh token)
//! 3. write the rotated tokens back, keeping a backup of the original file
//! 4. ask the regional personalisation API for the equipped card id
//! 5. download that card's art from valorant-api.com and save it as the cover
//!
//! When the Riot Client is already open, the token exchange is skipped: the
//! client's own local API hands out its current access token and entitlement,
//! so nothing on disk is rotated and the client can stay running. Saved
//! accounts' tokens are never read.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine as _;
use serde_json::{Value, json};

use crate::engine;
use crate::http;
use crate::platforms::Platform;

const RIOT_PROCESSES: [&str; 5] = [
    "RiotClientServices.exe",
    "RiotClientUx.exe",
    "RiotClientUxRender.exe",
    "VALORANT.exe",
    "LeagueClient.exe",
];

/// The game reports itself as a PC client. Riot's API rejects requests without
/// this header, and the value is the same on every Windows machine.
const CLIENT_PLATFORM: &str = "eyJwbGF0Zm9ybVR5cGUiOiJQQyIsInBsYXRmb3JtT1MiOiJXaW5kb3dzIiwicGxhdGZvcm1PU1ZlcnNpb24iOiIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwicGxhdGZvcm1DaGlwc2V0IjoiVW5rbm93biJ9";

struct Session {
    path: PathBuf,
    text: String,
    id_token: String,
    refresh_token: String,
    puuid: String,
    game_name: String,
    tag_line: String,
}

impl Session {
    fn display_name(&self) -> String {
        match (self.game_name.is_empty(), self.tag_line.is_empty()) {
            (false, false) => format!("{}#{}", self.game_name, self.tag_line),
            (false, true) => self.game_name.clone(),
            _ => "the signed-in Riot account".to_string(),
        }
    }
}

fn settings_path() -> PathBuf {
    crate::config::localappdata()
        .join("Riot Games")
        .join("Riot Client")
        .join("Data")
        .join("RiotGamesPrivateSettings.yaml")
}

fn lockfile_path() -> PathBuf {
    crate::config::localappdata()
        .join("Riot Games")
        .join("Riot Client")
        .join("Config")
        .join("lockfile")
}

/// The Riot Client lockfile is `name:pid:port:password:protocol`. Only the
/// port and password are needed to talk to the loopback API.
fn read_lockfile() -> Result<(u16, String), String> {
    let text = fs::read_to_string(lockfile_path()).map_err(|_| {
        "The Riot Client is not running, so its local sign-in is not available.".to_string()
    })?;
    let mut parts = text.trim().split(':');
    let _name = parts.next();
    let _pid = parts.next();
    let port = parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or("The Riot Client lockfile has no port.")?;
    let password = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or("The Riot Client lockfile has no password.")?;
    Ok((port, password.to_string()))
}

fn local_token() -> Result<(String, Option<String>), String> {
    let (port, password) = read_lockfile()?;
    let authorization = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("riot:{password}"))
    );
    let headers = [
        ("Authorization", authorization.as_str()),
        ("Accept", "application/json"),
    ];
    let base = format!("https://127.0.0.1:{port}");

    // v1 returns the access token and entitlement together. It is the fastest
    // path and avoids a second round trip.
    let response = http::request_local(
        "GET",
        &format!("{base}/entitlements/v1/token"),
        &headers,
        None,
    )?;
    if response.status == 200 {
        if let Ok(value) = serde_json::from_slice::<Value>(&response.body) {
            if let Some(access) = value
                .get("accessToken")
                .and_then(Value::as_str)
                .filter(|token| !token.is_empty())
            {
                let entitlement = value
                    .get("token")
                    .and_then(Value::as_str)
                    .filter(|token| !token.is_empty())
                    .map(str::to_string);
                return Ok((access.to_string(), entitlement));
            }
        }
    }

    // Older clients only expose the RSO access token here. The entitlement is
    // fetched separately, which still never touches the refresh token.
    let response = http::request_local(
        "GET",
        &format!("{base}/rso-auth/v1/authorization/access-token"),
        &headers,
        None,
    )?;
    if response.status != 200 {
        return Err(format!(
            "The Riot Client's local API is not ready (HTTP {}). Open the Riot Client window and try again.",
            response.status
        ));
    }
    let text = String::from_utf8_lossy(&response.body);
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if let Some(access) = value
            .get("accessToken")
            .or_else(|| value.get("access_token"))
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
        {
            return Ok((access.to_string(), None));
        }
        if let Some(access) = value.as_str().filter(|token| !token.is_empty()) {
            return Ok((access.to_string(), None));
        }
    }
    if trimmed.is_empty() || trimmed.starts_with('{') {
        return Err("The Riot Client returned no access token.".into());
    }
    Ok((trimmed.to_string(), None))
}

fn scalar(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix(key)?.strip_prefix(':')?.trim();
        let value = rest
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(rest);
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    })
}

fn jwt_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let mut encoded = payload.replace('-', "+").replace('_', "/");
    while encoded.len() % 4 != 0 {
        encoded.push('=');
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn read_session() -> Result<Session, String> {
    let path = settings_path();
    let text = fs::read_to_string(&path).map_err(|_| {
        "The Riot Client has no saved sign-in yet. Open it and sign in first.".to_string()
    })?;
    let id_token = scalar(&text, "id_token")
        .ok_or("The Riot Client is not signed in (no id token).")?;
    let refresh_token = scalar(&text, "refresh_token")
        .ok_or("The Riot Client is not signed in (no refresh token).")?;
    let claims = jwt_claims(&id_token).ok_or("The Riot sign-in token could not be read.")?;
    let puuid = claims
        .get("sub")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if puuid.is_empty() {
        return Err("The Riot sign-in token has no player id.".into());
    }
    let account = claims.get("acct");
    Ok(Session {
        path,
        text,
        id_token,
        refresh_token,
        puuid,
        game_name: account
            .and_then(|value| value.get("game_name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        tag_line: account
            .and_then(|value| value.get("tag_line"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

fn process_running(image: &str) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let filter = format!("IMAGENAME eq {image}");
    let Ok(output) = Command::new("tasklist")
        .args(["/FI", &filter, "/NH", "/FO", "CSV"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout)
        .to_ascii_lowercase()
        .contains(&image.to_ascii_lowercase())
}

fn riot_running() -> bool {
    RIOT_PROCESSES.iter().any(|image| process_running(image))
}

/// Whether a card can be fetched right now, without touching any token.
pub fn status_json() -> Value {
    let running = riot_running();
    match read_session() {
        Ok(session) => json!({
            "installed": true,
            "running": running,
            "signedIn": true,
            "liveName": session.display_name(),
        }),
        Err(_) => json!({
            "installed": settings_path().is_file(),
            "running": running,
            "signedIn": false,
            "liveName": Value::Null,
        }),
    }
}

fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

fn json_of(response: http::Response, what: &str) -> Result<Value, String> {
    if response.status != 200 {
        let detail = String::from_utf8_lossy(&response.body);
        let snippet: String = detail.trim().chars().take(160).collect();
        let snippet = snippet.trim();
        return Err(if snippet.is_empty() {
            format!("{what} failed (HTTP {}).", response.status)
        } else {
            format!("{what} failed (HTTP {}): {snippet}", response.status)
        });
    }
    serde_json::from_slice(&response.body).map_err(|_| format!("{what} returned unreadable data."))
}

fn exchange_refresh_token(refresh_token: &str) -> Result<Value, String> {
    let body = format!(
        "grant_type=refresh_token&refresh_token={}&client_id=riot-client",
        urlencode(refresh_token)
    );
    let response = http::request(
        "POST",
        "https://auth.riotgames.com/token",
        &[
            ("Content-Type", "application/x-www-form-urlencoded"),
            ("Accept", "application/json"),
        ],
        Some(body.as_bytes()),
    )?;
    json_of(response, "The Riot token exchange")
}

/// Replace a quoted scalar in the YAML while leaving every other byte alone.
fn replace_scalar(text: &str, key: &str, value: &str) -> String {
    let mut out = String::with_capacity(text.len() + value.len());
    for chunk in text.split_inclusive('\n') {
        let (body, ending) = if let Some(body) = chunk.strip_suffix("\r\n") {
            (body, "\r\n")
        } else if let Some(body) = chunk.strip_suffix('\n') {
            (body, "\n")
        } else {
            (chunk, "")
        };
        let trimmed = body.trim_start();
        if trimmed.starts_with(&format!("{key}:")) {
            let indent = &body[..body.len() - trimmed.len()];
            out.push_str(&format!("{indent}{key}: \"{value}\"{ending}"));
        } else {
            out.push_str(chunk);
        }
    }
    out
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("yaml.nexulon-backup")
}

fn write_tokens(session: &Session, id_token: &str, refresh_token: &str) -> Result<(), String> {
    let updated = replace_scalar(
        &replace_scalar(&session.text, "id_token", id_token),
        "refresh_token",
        refresh_token,
    );
    let backup = backup_path(&session.path);
    fs::copy(&session.path, &backup)
        .map_err(|error| format!("Could not back up the Riot settings: {error}"))?;
    let temporary = session.path.with_extension("yaml.nexulon-tmp");
    fs::write(&temporary, updated)
        .map_err(|error| format!("Could not write the new Riot tokens: {error}"))?;
    if let Err(error) = fs::rename(&temporary, &session.path) {
        let _ = fs::copy(&backup, &session.path);
        return Err(format!(
            "Could not save the new Riot tokens ({error}). The old file was restored."
        ));
    }
    Ok(())
}

fn restore_backup(path: &Path) {
    let backup = backup_path(path);
    if backup.is_file() {
        let _ = fs::copy(&backup, path);
    }
}

fn affinity(access_token: &str) -> Result<String, String> {
    let authorization = bearer(access_token);
    let response = http::request(
        "GET",
        "https://riot-geo.pas.si.riotgames.com/pas/v1/product/valorant",
        &[("Authorization", &authorization), ("Accept", "application/json")],
        None,
    )?;
    let value = json_of(response, "The Riot region lookup")?;
    value
        .get("affinities")
        .and_then(|value| value.get("live").or_else(|| value.get("event")))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "Riot did not report a region for this account.".to_string())
}

fn loadout(
    access_token: &str,
    entitlement: &str,
    version: &str,
    puuid: &str,
    region: &str,
) -> Result<Value, String> {
    let url = format!(
        "https://pd.{region}.a.pvp.net/personalization/v3/players/{puuid}/playerloadout"
    );
    let authorization = bearer(access_token);
    let response = http::request(
        "GET",
        &url,
        &[
            ("Authorization", &authorization),
            ("X-Riot-Entitlements-JWT", entitlement),
            ("X-Riot-ClientPlatform", CLIENT_PLATFORM),
            ("X-Riot-ClientVersion", version),
            ("Accept", "application/json"),
        ],
        None,
    )?;
    json_of(response, "The Valorant loadout lookup")
}

/// VALORANT rejects API calls unless they carry the game's own build number.
/// The local game log knows it exactly; valorant-api.com is the fallback.
fn client_version() -> Result<String, String> {
    let log = crate::config::localappdata()
        .join("VALORANT")
        .join("Saved")
        .join("Logs")
        .join("ShooterGame.log");
    if let Ok(text) = fs::read_to_string(&log) {
        for line in text.lines().rev() {
            if let Some(index) = line.find("CI server version:") {
                if let Some(version) = line[index + 18..].split_whitespace().next() {
                    if version.starts_with("release-") {
                        return Ok(version.to_string());
                    }
                }
            }
        }
    }
    let response = http::request(
        "GET",
        "https://valorant-api.com/v1/version",
        &[("Accept", "application/json")],
        None,
    )?;
    let value = json_of(response, "The Valorant version lookup")?;
    value
        .get("data")
        .and_then(|data| data.get("riotClientVersion"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "The installed VALORANT version could not be read.".to_string())
}

/// The personalisation API wants a second token, issued for the account's
/// entitlements, next to the bearer.
fn entitlements(access_token: &str) -> Result<String, String> {
    let authorization = bearer(access_token);
    let response = http::request(
        "POST",
        "https://entitlements.auth.riotgames.com/api/token/v1",
        &[
            ("Authorization", &authorization),
            ("Content-Type", "application/json"),
            ("Accept", "application/json"),
        ],
        Some(b"{}"),
    )?;
    let value = json_of(response, "The Riot entitlement lookup")?;
    value
        .get("entitlements_token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "Riot returned no entitlement token.".to_string())
}

fn equipped_card(value: &Value) -> Result<String, String> {
    value
        .get("Identity")
        .and_then(|value| value.get("PlayerCardID"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "The loadout has no player card.".to_string())
}

/// The region service is the fastest way to learn a player's shard, but it has
/// been returning odd errors for some accounts. Fall back to asking each shard
/// for the loadout until one answers.
/// The Riot ID and PUUID currently saved in the client, without rotating any
/// token. Used to warn before a refresh would overwrite another account.
pub fn signed_in() -> Result<(String, String), String> {
    let session = read_session()?;
    Ok((session.display_name(), session.puuid))
}

/// A validated live Riot session: one token exchange covers every call the
/// caller needs, so a refresh never rotates the token more than once.
pub struct Live {
    pub access_token: String,
    pub entitlement: String,
    pub version: String,
    pub puuid: String,
    pub live_name: String,
}

/// Read the signed-in client and return what the personalisation and MMR APIs
/// need. While Riot is running this uses the client's local API, so the client
/// keeps its own tokens; otherwise it performs one refresh-token exchange.
pub fn live() -> Result<Live, String> {
    let session = read_session()?;
    if riot_running() {
        return live_local(&session).map_err(|error| {
            format!(
                "{error} Nexulon tried the Riot Client's local sign-in so the client could stay open."
            )
        });
    }
    let live_name = session.display_name();

    let token = exchange_refresh_token(&session.refresh_token)?;
    let access_token = token
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or("Riot returned no access token.")?
        .to_string();
    let new_id = token
        .get("id_token")
        .and_then(Value::as_str)
        .unwrap_or(&session.id_token);
    let new_refresh = token
        .get("refresh_token")
        .and_then(Value::as_str)
        .ok_or("Riot returned no refresh token.")?;

    if let Err(error) = write_tokens(&session, new_id, new_refresh) {
        restore_backup(&session.path);
        return Err(format!(
            "{error} Riot may ask you to sign in again the next time the client starts."
        ));
    }

    let entitlement = entitlements(&access_token)?;
    let version = client_version()?;
    Ok(Live {
        access_token,
        entitlement,
        version,
        puuid: session.puuid,
        live_name,
    })
}

/// Use the running Riot Client's own access token instead of exchanging the
/// refresh token. Nothing is written back, so the client never loses sync.
fn live_local(session: &Session) -> Result<Live, String> {
    let (access_token, entitlement) = local_token()?;
    let entitlement = match entitlement {
        Some(token) => token,
        None => entitlements(&access_token)?,
    };
    let version = client_version()?;
    Ok(Live {
        access_token,
        entitlement,
        version,
        puuid: session.puuid.clone(),
        live_name: session.display_name(),
    })
}

/// GET a `pd.<region>.a.pvp.net` endpoint with the headers Riot requires.
pub fn api_get(live: &Live, region: &str, path: &str) -> Result<Value, String> {
    let url = format!("https://pd.{region}.a.pvp.net{path}");
    let authorization = bearer(&live.access_token);
    let response = http::request(
        "GET",
        &url,
        &[
            ("Authorization", &authorization),
            ("X-Riot-Entitlements-JWT", &live.entitlement),
            ("X-Riot-ClientPlatform", CLIENT_PLATFORM),
            ("X-Riot-ClientVersion", &live.version),
            ("Accept", "application/json"),
        ],
        None,
    )?;
    json_of(response, "The Riot request")
}

pub fn loadout_anywhere(live: &Live) -> Result<(String, Value), String> {
    let mut regions: Vec<String> = Vec::new();
    if let Ok(region) = affinity(&live.access_token) {
        regions.push(region);
    }
    for region in ["ap", "na", "eu", "kr"] {
        if !regions.iter().any(|known| known.eq_ignore_ascii_case(region)) {
            regions.push(region.to_string());
        }
    }

    let mut errors: Vec<String> = Vec::new();
    for region in regions {
        match loadout(
            &live.access_token,
            &live.entitlement,
            &live.version,
            &live.puuid,
            &region,
        ) {
            Ok(value) => return Ok((region, value)),
            Err(error) => errors.push(format!("{region}: {error}")),
        }
    }
    Err(if errors.is_empty() {
        "The Valorant loadout lookup failed.".to_string()
    } else {
        errors.join(" | ")
    })
}

fn card_art(card_id: &str) -> Result<(String, String, Option<String>), String> {
    let url = format!("https://valorant-api.com/v1/playercards/{card_id}");
    let response = http::request("GET", &url, &[("Accept", "application/json")], None)?;
    let value = json_of(response, "The player card lookup")?;
    let data = value
        .get("data")
        .ok_or("That player card was not found.")?;
    let name = data
        .get("displayName")
        .and_then(Value::as_str)
        .unwrap_or("Valorant player card")
        .to_string();
    let image = data
        .get("wideArt")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .or_else(|| data.get("largeArt").and_then(Value::as_str))
        .or_else(|| data.get("displayIcon").and_then(Value::as_str))
        .ok_or("That player card has no artwork.")?;
    // displayIcon is the square crop, which makes a better avatar than the
    // wide banner. Fall back to smallArt when a card does not ship one.
    let avatar = data
        .get("displayIcon")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .or_else(|| data.get("smallArt").and_then(Value::as_str))
        .map(str::to_string);
    Ok((name, image.to_string(), avatar))
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    let response = http::request("GET", url, &[("Accept", "image/*")], None)?;
    if response.status != 200 {
        return Err(format!("The card art download failed (HTTP {}).", response.status));
    }
    Ok(response.body)
}

pub struct FetchedCard {
    pub live_name: String,
    pub card_name: String,
    pub cover: PathBuf,
    pub avatar: Option<PathBuf>,
}

/// Fetch the equipped card of the account signed in to the Riot Client and
/// store it as `account`'s cover. The card's square icon becomes the avatar,
/// replacing the initials that would otherwise sit on the banner.
pub fn fetch_card(platform: &Platform, account: &str) -> Result<FetchedCard, String> {
    let live = live()?;
    let (_, loadout) = loadout_anywhere(&live)?;
    let card_id = equipped_card(&loadout)?;
    let (card_name, art_url, avatar_url) = card_art(&card_id)?;
    let bytes = download(&art_url)?;
    let cover = engine::set_artwork_bytes(platform, account, "cover", &bytes, "png")?;

    let avatar = avatar_url
        .and_then(|url| download(&url).ok())
        .and_then(|bytes| engine::set_artwork_bytes(platform, account, "avatar", &bytes, "png").ok());

    Ok(FetchedCard {
        live_name: live.live_name,
        card_name,
        cover,
        avatar,
    })
}
