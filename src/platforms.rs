// Copyright (C) 2026 Nexulon Corp. All rights reserved.
// See LICENSE for the terms.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::config;

/// One entry from `Platforms.json`, kept as raw JSON so unknown keys survive.
#[derive(Clone)]
pub struct Platform {
    pub name: String,
    raw: Value,
}

impl Platform {
    fn new(name: String, raw: Value) -> Self {
        Self { name, raw }
    }

    fn str_key(&self, key: &str) -> String {
        self.raw
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    fn bool_key(&self, key: &str) -> bool {
        self.raw.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    fn arr_key(&self, key: &str) -> Vec<String> {
        self.raw
            .get(key)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn identifiers(&self) -> Vec<String> {
        self.arr_key("Identifiers")
            .into_iter()
            .map(|i| i.to_ascii_lowercase())
            .collect()
    }

    pub fn exe_default(&self) -> String {
        self.str_key("ExeLocationDefault")
    }

    pub fn exe_extra_args(&self) -> String {
        self.str_key("ExeExtraArgs")
    }

    pub fn exes_to_end(&self) -> Vec<String> {
        self.arr_key("ExesToEnd")
    }

    /// Ordered `(live path, path inside the account folder)` pairs.
    pub fn login_files(&self) -> Vec<(String, String)> {
        self.raw
            .get("LoginFiles")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn path_list_to_clear(&self) -> Vec<String> {
        self.arr_key("PathListToClear")
    }

    pub fn all_files_required(&self) -> bool {
        self.bool_key("AllFilesRequired")
    }

    pub fn reg_delete_on_clear(&self) -> bool {
        self.bool_key("RegDeleteOnClear")
    }

    pub fn unique_id_file(&self) -> String {
        self.str_key("UniqueIdFile")
    }

    pub fn unique_id_method(&self) -> String {
        self.str_key("UniqueIdMethod")
    }

    pub fn unique_id_regex(&self) -> String {
        self.str_key("UniqueIdRegex")
    }

    // -- switching mode ---------------------------------------------------
    /// `copy` saves and restores login files; `profile` just relaunches the
    /// app with a chosen profile (browsers), so nothing is copied at all.
    pub fn switch_mode(&self) -> String {
        let mode = self.str_key("SwitchMode");
        if mode.is_empty() { "copy".to_string() } else { mode }
    }

    pub fn is_profile_mode(&self) -> bool {
        self.switch_mode().eq_ignore_ascii_case("profile")
    }

    // -- grouping ---------------------------------------------------------
    /// Broad service group used by the rail and the settings list. New
    /// definitions can set `"Category"`; older packs fall back to the name so
    /// an existing data folder still groups correctly after an update.
    pub fn category(&self) -> String {
        let explicit = self.str_key("Category");
        if !explicit.is_empty() {
            return normalize_category(&explicit);
        }
        infer_category(&self.name)
    }

    /// `chrome`, `firefox` or `dirs` — how the profile list is discovered.
    pub fn profile_list(&self) -> String {
        let kind = self.str_key("ProfileList");
        if kind.is_empty() { "dirs".to_string() } else { kind }
    }

    pub fn profile_root(&self) -> String {
        self.str_key("ProfileRoot")
    }

    /// Argument template; `{folder}` is the profile folder and `{path}` its
    /// absolute path.
    pub fn profile_arg(&self) -> String {
        self.str_key("ProfileArg")
    }

    pub fn cache_paths(&self) -> Vec<String> {
        self.raw
            .get("Extras")
            .and_then(|e| e.get("CachePaths"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    // -- settings ---------------------------------------------------------
    pub fn settings_path(&self) -> PathBuf {
        config::settings_dir().join(format!("{}.json", self.name))
    }

    pub fn settings(&self) -> Map<String, Value> {
        let mut defaults = default_settings();
        if let Ok(text) = std::fs::read_to_string(self.settings_path()) {
            if let Ok(Value::Object(stored)) = serde_json::from_str::<Value>(&text) {
                for (k, v) in stored {
                    defaults.insert(k, v);
                }
            }
        }
        defaults
    }

    pub fn save_settings(&self, settings: &Map<String, Value>) {
        let dir = config::settings_dir();
        let _ = std::fs::create_dir_all(&dir);
        let text = serde_json::to_string_pretty(&Value::Object(settings.clone()))
            .unwrap_or_else(|_| "{}".into());
        let _ = std::fs::write(self.settings_path(), text);
    }

    pub fn folder_path(&self) -> String {
        self.settings()
            .get("FolderPath")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    pub fn set_folder_path(&self, path: &str) {
        let mut settings = self.settings();
        settings.insert("FolderPath".into(), json!(path));
        self.save_settings(&settings);
    }

    /// Best-known launcher executable.
    pub fn exe_path(&self) -> String {
        let folder = crate::engine::expand(&self.folder_path(), Some(self));
        if !folder.is_empty() {
            let folder_path = Path::new(&folder);
            let exe_name = Path::new(&self.exe_default())
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let direct = folder_path.to_path_buf();
            if direct.is_file() {
                return direct.display().to_string();
            }
            let candidate = folder_path.join(&exe_name);
            if candidate.is_file() {
                return candidate.display().to_string();
            }
        }
        crate::engine::expand(&self.exe_default(), Some(self))
    }
}

pub fn default_settings() -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("FolderPath".into(), json!(""));
    map.insert("Basic_Admin".into(), json!(false));
    map.insert("ForgetAccountEnabled".into(), json!(false));
    map.insert("ShortcutsJson".into(), json!({}));
    map.insert("ClosingMethod".into(), json!("Combined"));
    map.insert("StartingMethod".into(), json!(""));
    map.insert("AutoStart".into(), json!(true));
    map.insert("ShowShortNotes".into(), json!(true));
    map.insert("AccountNotes".into(), json!({}));
    map
}

pub struct Book {
    platforms: Vec<Platform>,
    window: Value,
}

impl Book {
    pub fn load() -> Self {
        config::ensure_platforms();
        let live = config::platforms_json();
        let text = std::fs::read_to_string(&live)
            .ok()
            .filter(|t| serde_json::from_str::<Value>(t).is_ok())
            .unwrap_or_else(|| config::BUNDLED_PLATFORMS.to_string());

        let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);

        let platforms = parsed
            .get("Platforms")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .map(|(name, body)| Platform::new(name.clone(), body.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let window = std::fs::read_to_string(config::window_settings())
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .unwrap_or(Value::Null);

        Self { platforms, window }
    }

    pub fn get(&self, key: &str) -> Option<&Platform> {
        let key = key.trim();
        if key.is_empty() {
            return None;
        }
        let low = key.to_ascii_lowercase();
        self.platforms.iter().find(|p| p.name == key).or_else(|| {
            self.platforms.iter().find(|p| {
                p.name.to_ascii_lowercase() == low
                    || p.name.to_ascii_lowercase().replace(' ', "") == low.replace(' ', "")
                    || p.identifiers().iter().any(|i| *i == low)
            })
        })
    }

    fn enabled_ids(&self) -> Vec<String> {
        self.window
            .get("EnabledBasicPlatforms")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.to_ascii_lowercase())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn disabled_names(&self) -> Vec<String> {
        self.window
            .get("DisabledPlatforms")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Platforms the user enabled in the original app (all when unset).
    pub fn enabled(&self) -> Vec<&Platform> {
        let enabled_ids = self.enabled_ids();
        let disabled = self.disabled_names();
        self.platforms
            .iter()
            .filter(|p| !disabled.iter().any(|d| d == &p.name))
            .filter(|p| {
                enabled_ids.is_empty()
                    || p.identifiers().iter().any(|i| enabled_ids.contains(i))
            })
            .collect()
    }

    pub fn is_installed(&self, platform: &Platform) -> bool {
        let exe = platform.exe_path();
        !exe.is_empty() && Path::new(&exe).is_file()
    }
}

/// Map a hand-written category to the stable id the page understands.
fn normalize_category(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "game launchers" | "launchers" | "launcher" | "game-launchers" => "launcher",
        "games" | "game" => "game",
        "browsers" | "browser" => "browser",
        "ai agents" | "ai" | "agents" | "assistants" => "ai",
        "communication" | "chat" | "messaging" => "chat",
        "cloud & productivity" | "cloud" | "productivity" | "cloud-productivity" => "cloud",
        "creation" | "creative" | "creation & media" => "create",
        "media" | "music" | "streaming" => "media",
        _ => "other",
    }
    .to_string()
}

/// Best-effort grouping for platform packs that predate the `Category` key.
fn infer_category(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    let category = if [
        "chrome", "firefox", "edge", "brave", "vivaldi", "chromium", "librewolf",
        "waterfox", "opera", "arc browser",
    ]
    .iter()
    .any(|needle| name.contains(needle))
    {
        "browser"
    } else if [
        "claude", "chatgpt", "copilot", "cursor", "windsurf", "perplexity", "gemini",
        "grok", "openai",
    ]
    .iter()
    .any(|needle| name.contains(needle))
    {
        "ai"
    } else if [
        "discord", "slack", "teams", "telegram", "signal", "whatsapp", "zoom", "messenger",
    ]
    .iter()
    .any(|needle| name.contains(needle))
    {
        "chat"
    } else if ["notion", "obsidian", "dropbox", "google drive", "onedrive", "evernote"]
        .iter()
        .any(|needle| name.contains(needle))
    {
        "cloud"
    } else if ["obs", "photoshop", "blender", "davinci", "audacity", "figma"]
        .iter()
        .any(|needle| name.contains(needle))
    {
        "create"
    } else if ["spotify", "netflix", "vlc", "media player"]
        .iter()
        .any(|needle| name.contains(needle))
    {
        "media"
    } else if [
        "steam", "epic", "gog", "origin", "ea desktop", "ubisoft", "battlenet", "riot",
        "rockstar", "geforce", "jagex", "amazon games", "itch", "xbox", "hoyoplay",
        "wargaming", "bethesda", "paradox",
    ]
    .iter()
    .any(|needle| name.contains(needle))
    {
        "launcher"
    } else if [
        "tarkov", "genshin", "magic arena", "albion", "honkai", "star rail", "arena breakout",
        "delta force", "remote play", "roblox", "valorant", "league of legends", "fortnite",
    ]
    .iter()
    .any(|needle| name.contains(needle))
    {
        "game"
    } else {
        "other"
    };
    category.to_string()
}
