//! Friendly names and pictures read from the apps themselves.
//!
//! These are local-only: no network, no tokens. They are used as a fallback
//! when the user has not chosen artwork for an account, and as the small
//! "who is this really" line on a card (Steam persona, Google name, Claude
//! email). Anything that cannot be read is simply left empty.

use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value;

use crate::config;
use crate::engine;
use crate::platforms::Platform;

pub struct Meta {
    pub name: Option<String>,
    pub detail: Option<String>,
    pub avatar: Option<PathBuf>,
}

impl Meta {
    fn empty() -> Self {
        Self {
            name: None,
            detail: None,
            avatar: None,
        }
    }
}

pub fn for_account(platform: &Platform, account: &str) -> Meta {
    match platform.name.as_str() {
        "Steam" => steam(platform, account),
        "Chrome" => chrome(platform, account),
        "Firefox" => firefox(platform, account),
        "Claude Code" => claude(platform, account),
        "Claude Desktop" => claude_desktop(platform, account),
        _ => Meta::empty(),
    }
}

fn non_empty(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn read_text(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn steam(platform: &Platform, account: &str) -> Meta {
    let mut meta = Meta::empty();
    let id = engine::account_id(platform, account);
    if id.is_empty() {
        return meta;
    }
    let folder = PathBuf::from(platform.folder_path());
    let users = folder.join("config").join("loginusers.vdf");
    if let Some(text) = read_text(&users) {
        if let Ok(block) = Regex::new(&format!(r#"(?s)"{}"\s*\{{(.*?)\}}"#, regex::escape(&id))) {
            if let Some(captures) = block.captures(&text) {
                let body = captures.get(1).map(|match_| match_.as_str()).unwrap_or_default();
                if let Ok(persona) = Regex::new(r#""PersonaName"\s*"([^"]*)""#) {
                    if let Some(name) = persona
                        .captures(body)
                        .and_then(|captures| captures.get(1))
                        .and_then(|match_| non_empty(match_.as_str()))
                    {
                        meta.name = Some(name);
                    }
                }
            }
        }
    }

    // Steam keeps downloaded avatars in config\avatarcache when it has them.
    let cache = folder.join("config").join("avatarcache");
    if let Ok(entries) = fs::read_dir(&cache) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_ascii_lowercase();
            if name.starts_with(&id) && (lower.ends_with(".png") || lower.ends_with(".jpg")) {
                meta.avatar = Some(entry.path());
                break;
            }
        }
    }
    meta
}

fn chrome(platform: &Platform, account: &str) -> Meta {
    let mut meta = Meta::empty();
    let root = PathBuf::from(platform.profile_root());
    let state = root.join("Local State");
    if let Some(text) = read_text(&state) {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            if let Some(info) = value
                .get("profile")
                .and_then(|profile| profile.get("info_cache"))
                .and_then(|cache| cache.get(account))
            {
                let pick = |key: &str| {
                    info.get(key)
                        .and_then(Value::as_str)
                        .and_then(non_empty)
                };
                meta.name = pick("gaia_name")
                    .or_else(|| pick("gaia_given_name"))
                    .or_else(|| pick("user_name"));
                if let (Some(name), Some(email)) = (meta.name.clone(), pick("user_name")) {
                    if name != email {
                        meta.detail = Some(email);
                    }
                }
            }
        }
    }

    let picture = root.join(account).join("Google Profile Picture.png");
    if picture.is_file() {
        meta.avatar = Some(picture);
    }
    meta
}

fn firefox(platform: &Platform, account: &str) -> Meta {
    let mut meta = Meta::empty();
    let root = PathBuf::from(platform.profile_root());
    let prefs = root.join(account).join("prefs.js");
    if let Some(text) = read_text(&prefs) {
        if let Ok(pattern) =
            Regex::new(r#"user_pref\("identity\.fxaccounts\.account\.device\.name",\s*"([^"]*)"\)"#)
        {
            if let Some(name) = pattern
                .captures(&text)
                .and_then(|captures| captures.get(1))
                .and_then(|match_| non_empty(match_.as_str()))
            {
                meta.detail = Some(name.clone());
                meta.name = Some(name);
            }
        }
    }
    meta
}

fn claude(platform: &Platform, account: &str) -> Meta {
    let mut meta = Meta::empty();

    // The saved session tells us the plan, and its account id lets us decide
    // whether the live profile belongs to this account.
    let saved = engine::account_dir(platform, account).join(".credentials.json");
    let mut saved_uuid: Option<String> = None;
    if let Some(text) = read_text(&saved) {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            if let Some(oauth) = value.get("claudeAiOauth") {
                if let Some(plan) = oauth
                    .get("subscriptionType")
                    .and_then(Value::as_str)
                    .and_then(non_empty)
                {
                    meta.detail = Some(plan);
                }
                saved_uuid = oauth
                    .get("accountUuid")
                    .and_then(Value::as_str)
                    .and_then(non_empty);
            }
        }
    }

    // Only borrow the live email when the live account is this saved account.
    if let Some(text) = read_text(&config::home().join(".claude.json")) {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            let live = value.get("oauthAccount");
            let live_uuid = live
                .and_then(|account| account.get("accountUuid"))
                .and_then(Value::as_str)
                .and_then(non_empty);
            let matches = match (&saved_uuid, &live_uuid) {
                (Some(saved), Some(live)) => saved == live,
                _ => false,
            };
            if matches {
                if let Some(email) = live
                    .and_then(|account| account.get("emailAddress"))
                    .and_then(Value::as_str)
                    .and_then(non_empty)
                {
                    meta.name = Some(email);
                }
            }
        }
    }
    meta
}

fn claude_desktop(platform: &Platform, account: &str) -> Meta {
    let mut meta = Meta::empty();
    let saved = engine::account_dir(platform, account).join("config.json");
    if let Some(text) = read_text(&saved) {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            if value
                .get("lastKnownAccountUuid")
                .and_then(Value::as_str)
                .and_then(non_empty)
                .is_some()
            {
                meta.detail = Some("Claude Desktop".to_string());
            }
        }
    }
    meta
}
