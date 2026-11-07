use std::path::{Path, PathBuf};

fn env_path(name: &str, fallback: impl FnOnce() -> PathBuf) -> PathBuf {
    match std::env::var_os(name) {
        Some(value) => PathBuf::from(value),
        None => fallback(),
    }
}

pub fn home() -> PathBuf {
    env_path("USERPROFILE", || PathBuf::from(r"C:\Users\Default"))
}

pub fn appdata() -> PathBuf {
    env_path("APPDATA", || home().join(r"AppData\Roaming"))
}

pub fn localappdata() -> PathBuf {
    env_path("LOCALAPPDATA", || home().join(r"AppData\Local"))
}

pub fn programdata() -> PathBuf {
    env_path("PROGRAMDATA", || PathBuf::from(r"C:\ProgramData"))
}

pub fn programfiles() -> PathBuf {
    env_path("ProgramFiles", || PathBuf::from(r"C:\Program Files"))
}

pub fn programfiles_x86() -> PathBuf {
    env_path("ProgramFiles(x86)", || PathBuf::from(r"C:\Program Files (x86)"))
}

pub fn documents() -> PathBuf {
    home().join("Documents")
}

pub fn start_menu_programdata() -> PathBuf {
    programdata().join(r"Microsoft\Windows\Start Menu\Programs")
}

/// Everything this app owns lives in one folder, separate from TcNo.
/// `NEXULON_DATA_DIR` overrides it (used by the sandbox self-test).
pub fn data_root() -> PathBuf {
    env_path("NEXULON_DATA_DIR", || appdata().join("Nexulon Switcher"))
}

/// The old TcNo folder is only read once, during first-run import.
pub fn legacy_root() -> PathBuf {
    appdata().join("TcNo Account Switcher")
}

pub fn login_cache() -> PathBuf {
    data_root().join("LoginCache")
}

pub fn settings_dir() -> PathBuf {
    data_root().join("Settings")
}

pub fn window_settings() -> PathBuf {
    data_root().join("WindowSettings.json")
}

pub fn platforms_json() -> PathBuf {
    data_root().join("Platforms.json")
}

/// Per-account artwork (avatars and card covers) the user picked.
pub fn artwork_dir() -> PathBuf {
    data_root().join("Artwork")
}

/// WebView2 keeps a browser profile on disk. It must live somewhere the user
/// can write, because a per-machine install sits in `Program Files`.
pub fn webview_data_dir() -> PathBuf {
    localappdata().join("Nexulon Switcher").join("WebView2")
}

pub fn log_path() -> PathBuf {
    data_root().join("logs").join("nexulon-switcher.log")
}

/// Create the Nexulon data folder on first launch and, if the old TcNo folder
/// exists, import its login cache/settings once. After this the app never reads
/// the TcNo folder again.
pub fn ensure_data_root() {
    if std::env::var_os("NEXULON_DATA_DIR").is_some() {
        return;
    }
    let root = data_root();
    let marker = root.join(".initialized");
    if marker.is_file() {
        return;
    }
    let _ = std::fs::create_dir_all(&root);

    let legacy = legacy_root();
    if legacy.is_dir() {
        for name in ["Platforms.json", "WindowSettings.json"] {
            let source = legacy.join(name);
            if source.is_file() {
                let _ = std::fs::copy(&source, root.join(name));
            }
        }
        for name in ["LoginCache", "Settings"] {
            let source = legacy.join(name);
            if source.is_dir() {
                copy_dir(&source, &root.join(name));
            }
        }
    }

    let _ = std::fs::write(
        &marker,
        "Nexulon Switcher data folder. Safe to delete to start fresh.\n",
    );
}

fn copy_dir(source: &Path, target: &Path) {
    let _ = std::fs::create_dir_all(target);
    let Ok(entries) = std::fs::read_dir(source) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let destination = target.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &destination);
        } else {
            let _ = std::fs::copy(&path, &destination);
        }
    }
}

/// Shipped platform definitions, compiled into the exe as the fallback.
pub const BUNDLED_PLATFORMS: &str = include_str!("../data/Platforms.default.json");

/// Add platforms that shipped after the user's `Platforms.json` was created.
///
/// Existing entries are never touched, so a hand-edited folder or identifier
/// survives an update; only names that are missing are copied in.
pub fn ensure_platforms() {
    let bundled: serde_json::Value = match serde_json::from_str(BUNDLED_PLATFORMS) {
        Ok(value) => value,
        Err(_) => return,
    };
    let path = platforms_json();
    let existing = std::fs::read_to_string(&path).ok();
    let mut live: serde_json::Value = existing
        .as_deref()
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or_else(|| bundled.clone());

    let Some(bundled_platforms) = bundled.get("Platforms").and_then(|v| v.as_object()) else {
        return;
    };
    let Some(root) = live.as_object_mut() else {
        return;
    };
    if !root.get("Platforms").map(|v| v.is_object()).unwrap_or(false) {
        root.insert("Platforms".into(), serde_json::json!({}));
    }
    let Some(live_platforms) = root.get_mut("Platforms").and_then(|v| v.as_object_mut()) else {
        return;
    };

    let mut added = 0;
    let mut added_ids: Vec<String> = Vec::new();
    for (name, body) in bundled_platforms {
        if !live_platforms.contains_key(name) {
            live_platforms.insert(name.clone(), body.clone());
            if let Some(id) = body
                .get("Identifiers")
                .and_then(|value| value.as_array())
                .and_then(|items| items.first())
                .and_then(|value| value.as_str())
            {
                added_ids.push(id.to_string());
            }
            added += 1;
        }
    }
    if added == 0 && existing.is_some() {
        return;
    }

    let _ = std::fs::create_dir_all(data_root());
    if let Ok(text) = serde_json::to_string_pretty(&live) {
        let _ = std::fs::write(&path, text);
    }
    enable_platforms(&added_ids);
}

/// A platform that ships later must also be switched on in the old window
/// settings, otherwise the rail would hide it behind the legacy filter.
fn enable_platforms(ids: &[String]) {
    if ids.is_empty() {
        return;
    }
    let path = window_settings();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    let Some(root) = value.as_object_mut() else {
        return;
    };
    let Some(list) = root
        .get_mut("EnabledBasicPlatforms")
        .and_then(|value| value.as_array_mut())
    else {
        return;
    };
    let mut changed = false;
    for id in ids {
        if !list.iter().any(|value| value.as_str() == Some(id.as_str())) {
            list.push(serde_json::json!(id));
            changed = true;
        }
    }
    if changed {
        if let Ok(text) = serde_json::to_string_pretty(&value) {
            let _ = std::fs::write(&path, text);
        }
    }
}

pub const ID_FILE: &str = "TcNoAccSwitcher-ID.instance";
pub const REG_FILE: &str = "reg.json";

pub fn is_non_account(entry: &str) -> bool {
    matches!(
        entry.to_ascii_lowercase().as_str(),
        "shortcuts" | "order.json" | "ids.json" | "reg.json"
    )
}
