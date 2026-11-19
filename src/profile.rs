//! Cached Riot profile data: competitive rank, peak, account level and the
//! equipped title, banner and sprays.
//!
//! Everything here is fetched with the *live* Riot Client token, exactly like
//! the player card, and stored per account so the card can show a rank badge
//! without another token exchange. Refreshing is always explicit because each
//! refresh rotates the client's refresh token.
//!
//! Public valorant-api.com data (tier names, rank icons, season names) is
//! cached for a week; it needs no Riot sign-in.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use crate::config;
use crate::http;
use crate::platforms::Platform;
use crate::riot;

const CATALOG_MAX_AGE: u64 = 7 * 24 * 60 * 60;
/// Bumped when the catalog layout changes, so an old cache is rebuilt.
const CATALOG_VERSION: u64 = 3;

/// `DIAMOND 1` -> `Diamond 1`, `ACT III` -> `Act III`, `EPISODE 1` -> `Episode 1`.
/// Roman numerals keep their capitals.
fn pretty_name(text: &str) -> String {
    let roman = |word: &str| {
        !word.is_empty()
            && word.len() <= 5
            && word
                .chars()
                .all(|ch| "IVXLCDMivxlcdm".contains(ch))
    };
    let mut out: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        if roman(word) {
            out.push(word.to_uppercase());
            continue;
        }
        let mut chars = word.chars();
        let Some(first) = chars.next() else {
            continue;
        };
        let mut pretty = String::new();
        pretty.extend(first.to_uppercase());
        pretty.extend(chars.flat_map(char::to_lowercase));
        out.push(pretty);
    }
    out.join(" ")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or(0)
}

fn platform_dir(platform: &Platform) -> PathBuf {
    config::data_root().join("Profiles").join(&platform.name)
}

fn index_path(platform: &Platform) -> PathBuf {
    platform_dir(platform).join("index.json")
}

fn catalog_path() -> PathBuf {
    config::data_root()
        .join("Profiles")
        .join("Riot Games")
        .join("catalog.json")
}

fn read_index(platform: &Platform) -> Map<String, Value> {
    fs::read_to_string(index_path(platform))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default()
}

fn write_index(platform: &Platform, index: &Map<String, Value>) {
    let path = index_path(platform);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let text = serde_json::to_string(&Value::Object(index.clone())).unwrap_or_else(|_| "{}".into());
    let _ = fs::write(path, text);
}

pub fn cached(platform: &Platform, name: &str) -> Option<Value> {
    read_index(platform).remove(name)
}

/// A compact form for the account list payload.
pub fn summary(platform: &Platform, name: &str) -> Option<Value> {
    let profile = cached(platform, name)?;
    let rank = profile.get("rank").and_then(Value::as_object);
    let tier = rank
        .and_then(|rank| rank.get("tier"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Some(json!({
        "tier": tier,
        "rank": rank.and_then(|rank| rank.get("name")).cloned().unwrap_or(Value::Null),
        "rr": rank.and_then(|rank| rank.get("rr")).cloned().unwrap_or(Value::Null),
        "peakTier": rank.and_then(|rank| rank.get("peakTier")).cloned().unwrap_or(Value::Null),
        "peak": rank.and_then(|rank| rank.get("peak")).cloned().unwrap_or(Value::Null),
        "level": profile.get("level").cloned().unwrap_or(Value::Null),
        "fetchedAt": profile.get("fetchedAt").cloned().unwrap_or(Value::Null),
        "icon": rank_icon_path(tier).and_then(|path| data_uri(&path)),
    }))
}

/// A rank icon that is already on disk. Listing accounts must never wait on
/// the network, so missing icons are left for the next profile refresh.
fn rank_icon_path(tier: u64) -> Option<PathBuf> {
    if tier == 0 {
        return None;
    }
    let path = config::data_root()
        .join("Profiles")
        .join("Riot Games")
        .join(format!("rank-{tier}.png"));
    path.is_file().then_some(path)
}

fn data_uri(path: &std::path::Path) -> Option<String> {
    use base64::Engine;
    let bytes = fs::read(path).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}

/// Move a cached profile when its account is renamed.
pub fn rekey(platform: &Platform, old: &str, new: &str) {
    let mut index = read_index(platform);
    if let Some(entry) = index.remove(old) {
        index.insert(new.to_string(), entry);
        write_index(platform, &index);
    }
}

/// Rank icon for a tier, downloading it from valorant-api.com once.
pub fn rank_icon(tier: u64) -> Option<PathBuf> {
    if tier == 0 {
        return None;
    }
    let path = config::data_root()
        .join("Profiles")
        .join("Riot Games")
        .join(format!("rank-{tier}.png"));
    if path.is_file() {
        return Some(path);
    }
    let catalog = load_catalog(false).ok()?;
    let url = catalog
        .get("tiers")
        .and_then(|tiers| tiers.get(tier.to_string()))
        .and_then(|entry| entry.get("icon"))
        .and_then(Value::as_str)?;
    let response = http::request("GET", url, &[("Accept", "image/*")], None).ok()?;
    if response.status != 200 {
        return None;
    }
    fs::create_dir_all(path.parent()?).ok()?;
    fs::write(&path, response.body).ok()?;
    Some(path)
}

fn load_catalog(force: bool) -> Result<Value, String> {
    if !force {
        if let Ok(text) = fs::read_to_string(catalog_path()) {
            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                let age = now().saturating_sub(value.get("fetchedAt").and_then(Value::as_u64).unwrap_or(0));
                if age < CATALOG_MAX_AGE
                    && value.get("version").and_then(Value::as_u64) == Some(CATALOG_VERSION)
                    && value.get("tiers").is_some()
                {
                    return Ok(value);
                }
            }
        }
    }

    let tiers_response = http::request(
        "GET",
        "https://valorant-api.com/v1/competitivetiers",
        &[("Accept", "application/json")],
        None,
    )?;
    if tiers_response.status != 200 {
        return Err(format!(
            "The rank list download failed (HTTP {}).",
            tiers_response.status
        ));
    }
    let tiers_value: Value = serde_json::from_slice(&tiers_response.body)
        .map_err(|_| "The rank list returned unreadable data.".to_string())?;
    let sets = tiers_value
        .get("data")
        .and_then(Value::as_array)
        .ok_or("The rank list was empty.")?;
    let latest = sets.last().ok_or("The rank list was empty.")?;

    let mut tiers = Map::new();
    for tier in latest.get("tiers").and_then(Value::as_array).into_iter().flatten() {
        let number = tier.get("tier").and_then(Value::as_u64).unwrap_or(0);
        let name = pretty_name(
            tier.get("tierName")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .or_else(|| tier.get("divisionName").and_then(Value::as_str))
                .unwrap_or("Unranked"),
        );
        let icon = tier
            .get("smallIcon")
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
            .map(str::to_string);
        tiers.insert(
            number.to_string(),
            json!({"name": name, "icon": icon}),
        );
    }

    let mut seasons = Map::new();
    if let Ok(response) = http::request(
        "GET",
        "https://valorant-api.com/v1/seasons",
        &[("Accept", "application/json")],
        None,
    ) {
        if response.status == 200 {
            if let Ok(value) = serde_json::from_slice::<Value>(&response.body) {
                let data = value
                    .get("data")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let mut episodes = Map::new();
                for season in &data {
                    let Some(id) = season.get("uuid").and_then(Value::as_str) else {
                        continue;
                    };
                    if season.get("type").and_then(Value::as_str).is_none() {
                        if let Some(name) = season.get("displayName").and_then(Value::as_str) {
                            episodes.insert(id.to_string(), json!(pretty_name(name)));
                        }
                    }
                }
                for (order, season) in data.iter().enumerate() {
                    if season.get("type").and_then(Value::as_str) != Some("EAresSeasonType::Act") {
                        continue;
                    }
                    let Some(id) = season.get("uuid").and_then(Value::as_str) else {
                        continue;
                    };
                    let act = pretty_name(
                        season
                            .get("displayName")
                            .and_then(Value::as_str)
                            .unwrap_or("Act"),
                    );
                    let parent = season
                        .get("parentUuid")
                        .and_then(Value::as_str)
                        .and_then(|parent| episodes.get(parent))
                        .and_then(Value::as_str);
                    let name = match parent {
                        Some(episode) => format!("{episode} · {act}"),
                        None => act,
                    };
                    seasons.insert(id.to_string(), json!({"name": name, "order": order}));
                }
            }
        }
    }

    let catalog = json!({
        "version": CATALOG_VERSION,
        "fetchedAt": now(),
        "tiers": tiers,
        "seasons": seasons,
    });
    if let Some(parent) = catalog_path().parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        catalog_path(),
        serde_json::to_string(&catalog).unwrap_or_else(|_| "{}".into()),
    );
    Ok(catalog)
}

fn tier_name(catalog: &Value, tier: u64) -> String {
    if tier == 0 {
        return "Unranked".to_string();
    }
    catalog
        .get("tiers")
        .and_then(|tiers| tiers.get(tier.to_string()))
        .and_then(|entry| entry.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("Unranked")
        .to_string()
}

fn season_name(catalog: &Value, id: &str) -> String {
    catalog
        .get("seasons")
        .and_then(|seasons| seasons.get(id))
        .and_then(|entry| entry.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown act")
        .to_string()
}

fn season_order(catalog: &Value, id: &str) -> u64 {
    catalog
        .get("seasons")
        .and_then(|seasons| seasons.get(id))
        .and_then(|entry| entry.get("order"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn as_u64(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn rank_from_mmr(mmr: &Value, catalog: &Value) -> Value {
    let seasons = mmr
        .get("QueueSkills")
        .and_then(|skills| skills.get("competitive"))
        .and_then(|competitive| competitive.get("SeasonalInfoBySeasonID"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let latest = mmr.get("LatestCompetitiveUpdate");
    let mut current_season = latest
        .and_then(|update| update.get("SeasonID"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default();
    if current_season.is_empty() {
        current_season = seasons.keys().last().cloned().unwrap_or_default();
    }

    let current = seasons.get(&current_season);
    let tier = latest
        .and_then(|update| update.get("TierAfterUpdate"))
        .and_then(Value::as_u64)
        .unwrap_or_else(|| current.map(|entry| as_u64(entry, "CompetitiveTier")).unwrap_or(0));
    let rr = latest
        .and_then(|update| update.get("RankedRatingAfterUpdate"))
        .and_then(Value::as_u64)
        .unwrap_or_else(|| current.map(|entry| as_u64(entry, "RankedRating")).unwrap_or(0));

    let mut peak_tier = 0;
    let mut peak_season = String::new();
    let mut history: Vec<Value> = Vec::new();
    for (id, entry) in &seasons {
        let season_tier = as_u64(entry, "CompetitiveTier");
        if season_tier > peak_tier {
            peak_tier = season_tier;
            peak_season = id.clone();
        }
        // Acts the account never played (tier 0) would just read "Unranked".
        if season_tier == 0 {
            continue;
        }
        history.push(json!({
            "id": id,
            "name": season_name(catalog, id),
            "order": season_order(catalog, id),
            "tier": season_tier,
            "rank": tier_name(catalog, season_tier),
            "rr": as_u64(entry, "RankedRating"),
            "wins": as_u64(entry, "NumberOfWins"),
        }));
    }
    if tier > peak_tier {
        peak_tier = tier;
        peak_season = current_season.clone();
    }
    history.sort_by(|left, right| {
        right
            .get("order")
            .and_then(Value::as_u64)
            .cmp(&left.get("order").and_then(Value::as_u64))
    });

    json!({
        "tier": tier,
        "name": tier_name(catalog, tier),
        "rr": rr,
        "peakTier": peak_tier,
        "peak": tier_name(catalog, peak_tier),
        "peakSeason": season_name(catalog, &peak_season),
        "season": season_name(catalog, &current_season),
        "history": history,
    })
}

fn expression_ids(loadout: &Value) -> Vec<String> {
    loadout
        .get("ActiveExpressions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("AssetID").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn public_name(kind: &str, id: &str) -> Option<String> {
    let url = format!("https://valorant-api.com/v1/{kind}/{id}");
    let response = http::request("GET", &url, &[("Accept", "application/json")], None).ok()?;
    if response.status != 200 {
        return None;
    }
    let value: Value = serde_json::from_slice(&response.body).ok()?;
    let data = value.get("data")?;
    data.get("displayName")
        .or_else(|| data.get("titleText"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Player titles read better as `Instinctual` than `Instinctual Title`.
fn public_title(id: &str) -> Option<String> {
    let url = format!("https://valorant-api.com/v1/playertitles/{id}");
    let response = http::request("GET", &url, &[("Accept", "application/json")], None).ok()?;
    if response.status != 200 {
        return None;
    }
    let value: Value = serde_json::from_slice(&response.body).ok()?;
    let data = value.get("data")?;
    data.get("titleText")
        .or_else(|| data.get("displayName"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// What a refresh produced: either fresh data, or the live account does not
/// match the saved account and the caller must confirm.
pub enum Outcome {
    Fetched(Value),
    Mismatch { live: String, saved: String },
}

pub(crate) fn same_account(platform: &Platform, account: &str, puuid: &str, live_name: &str) -> bool {
    let saved_id = crate::engine::account_id(platform, account);
    if !saved_id.is_empty() && saved_id.eq_ignore_ascii_case(puuid) {
        return true;
    }
    // Fall back to the folder name, which usually is the game name without tag.
    let saved = account.trim().to_ascii_lowercase();
    let live = live_name.trim().to_ascii_lowercase();
    let game = live.split('#').next().unwrap_or(&live);
    saved == live || saved == game
}

/// Fetch everything the card and profile drawer need with one token exchange.
///
/// `force` is false for a normal refresh: when the account signed in to the
/// Riot Client is not the account being refreshed, nothing is written and the
/// caller is told so. The UI asks the user before passing `force = true`.
pub fn fetch(platform: &Platform, account: &str, force: bool) -> Result<Outcome, String> {
    let (live_name, live_puuid) = riot::signed_in()?;
    if !force && !same_account(platform, account, &live_puuid, &live_name) {
        return Ok(Outcome::Mismatch {
            live: live_name,
            saved: account.to_string(),
        });
    }

    let live = riot::live()?;
    let (region, loadout) = riot::loadout_anywhere(&live)?;
    let catalog = load_catalog(false)?;

    let mmr = riot::api_get(&live, &region, &format!("/mmr/v1/players/{}", live.puuid)).ok();
    let xp = riot::api_get(
        &live,
        &region,
        &format!("/account-xp/v1/players/{}", live.puuid),
    )
    .ok();

    let rank = mmr
        .as_ref()
        .map(|mmr| rank_from_mmr(mmr, &catalog))
        .unwrap_or(Value::Null);

    // Fetch the current and peak rank icons now, while a refresh is already
    // doing network work, so listing accounts stays offline and instant.
    for tier in [
        rank.get("tier").and_then(Value::as_u64).unwrap_or(0),
        rank.get("peakTier").and_then(Value::as_u64).unwrap_or(0),
    ] {
        let _ = rank_icon(tier);
    }

    let level = xp
        .as_ref()
        .and_then(|xp| xp.get("Progress"))
        .and_then(|progress| progress.get("Level"))
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let identity = loadout.get("Identity");
    let title_id = identity
        .and_then(|identity| identity.get("PlayerTitleID"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let title = if title_id.is_empty() {
        Value::Null
    } else {
        public_title(title_id).map(Value::String).unwrap_or(Value::Null)
    };

    // Valorant has no separate banner; the equipped player card is the banner.
    let card_id = identity
        .and_then(|identity| identity.get("PlayerCardID"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let banner = if card_id.is_empty() {
        Value::Null
    } else {
        public_name("playercards", card_id)
            .map(Value::String)
            .unwrap_or(Value::Null)
    };

    let spray_ids = expression_ids(&loadout);
    let mut sprays: Vec<String> = Vec::new();
    for id in spray_ids.iter().take(3) {
        if let Some(name) = public_name("sprays", id) {
            sprays.push(name);
        }
    }

    let profile = json!({
        "name": account,
        "liveName": live.live_name,
        "puuid": live.puuid,
        "region": region,
        "fetchedAt": now(),
        "level": level,
        "rank": rank,
        "title": title,
        "banner": banner,
        "sprays": sprays,
    });

    let mut index = read_index(platform);
    index.insert(account.to_string(), profile.clone());
    write_index(platform, &index);
    Ok(Outcome::Fetched(profile))
}
