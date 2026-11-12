//! Bridge between the WebView and the switching engine.
//!
//! The page talks to Rust with `window.ipc.postMessage(JSON)` and receives a
//! single call back into `window.__nexulonReply(id, ok, payload)`.

use std::path::Path;
use std::sync::OnceLock;

use serde_json::{Map, Value, json};

use crate::config;
use crate::engine;
use crate::platforms::{Book, Platform};

static BOOK: OnceLock<Book> = OnceLock::new();

fn book() -> &'static Book {
    BOOK.get_or_init(Book::load)
}

fn platform(params: &Value) -> Result<&'static Platform, String> {
    let name = params
        .get("platform")
        .and_then(Value::as_str)
        .unwrap_or_default();
    book().get(name).ok_or_else(|| format!("Unknown platform: {name}"))
}

fn account_name(params: &Value) -> Result<String, String> {
    let name = params
        .get("account")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if name.trim().is_empty() {
        return Err("No account given.".into());
    }
    Ok(name)
}

fn flag(params: &Value, key: &str, fallback: bool) -> bool {
    params.get(key).and_then(Value::as_bool).unwrap_or(fallback)
}

// ---------------------------------------------------------------------------
// preferences (app-wide, independent of any single platform)
// ---------------------------------------------------------------------------
fn prefs_path() -> std::path::PathBuf {
    config::settings_dir().join("Nexulon Switcher.json")
}

fn load_prefs() -> Map<String, Value> {
    let mut prefs = Map::new();
    prefs.insert("clearOldLogin".into(), json!(true));
    prefs.insert("startLauncher".into(), json!(true));
    prefs.insert("ui".into(), json!({}));
    if let Ok(text) = std::fs::read_to_string(prefs_path()) {
        if let Ok(Value::Object(stored)) = serde_json::from_str::<Value>(&text) {
            for (key, value) in stored {
                if prefs.contains_key(&key) {
                    prefs.insert(key, value);
                }
            }
        }
    }
    prefs
}

fn set_prefs(params: &Value) -> Result<Value, String> {
    let mut prefs = load_prefs();
    for key in ["clearOldLogin", "startLauncher"] {
        if let Some(value) = params.get(key).and_then(Value::as_bool) {
            prefs.insert(key.into(), json!(value));
        }
    }
    if let Some(patch) = params.get("ui").and_then(Value::as_object) {
        let slot = prefs.entry("ui".to_string()).or_insert_with(|| json!({}));
        if !slot.is_object() {
            *slot = json!({});
        }
        if let Some(map) = slot.as_object_mut() {
            for (key, value) in patch {
                map.insert(key.clone(), value.clone());
            }
        }
    }
    let dir = config::settings_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&Value::Object(prefs.clone()))
        .map_err(|e| e.to_string())?;
    std::fs::write(prefs_path(), text).map_err(|e| e.to_string())?;
    Ok(Value::Object(prefs))
}

fn prefs_payload() -> Value {
    Value::Object(load_prefs())
}

// ---------------------------------------------------------------------------
// launch-at-login (per-user Run key, no elevation needed)
// ---------------------------------------------------------------------------
const AUTOSTART_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const AUTOSTART_NAME: &str = "Nexulon Switcher";

fn autostart_enabled() -> bool {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(AUTOSTART_KEY)
        .and_then(|key| key.get_value::<String, _>(AUTOSTART_NAME))
        .is_ok()
}

fn set_autostart(enabled: bool) -> Result<Value, String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(AUTOSTART_KEY, KEY_SET_VALUE)
        .map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let command = format!("\"{}\"", exe.display());
        key.set_value(AUTOSTART_NAME, &command)
            .map_err(|e| e.to_string())?;
    } else {
        match key.delete_value(AUTOSTART_NAME) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(json!({ "enabled": enabled }))
}

fn autostart(params: &Value) -> Result<Value, String> {
    match params.get("enabled").and_then(Value::as_bool) {
        Some(enabled) => set_autostart(enabled),
        None => Ok(json!({ "enabled": autostart_enabled() })),
    }
}

// ---------------------------------------------------------------------------
// asset helpers
// ---------------------------------------------------------------------------
fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

fn data_uri(path: &Path) -> Option<String> {
    use base64::Engine;
    let bytes = std::fs::read(path).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:{};base64,{}", mime_for(path), encoded))
}

fn platform_icon(platform: &Platform) -> Value {
    // The shipped artwork is monochrome. The page inlines it so CSS can tint
    // each logo to the active theme instead of drawing black on a dark panel.
    match engine::platform_icon(platform) {
        Some(svg) => json!(svg),
        None => Value::Null,
    }
}

fn platform_payload(platform: &Platform) -> Value {
    let accounts = engine::list_accounts(platform);
    json!({
        "name": platform.name,
        "icon": platform_icon(platform),
        "accounts": accounts.len(),
        "accountNames": accounts,
        "installed": book().is_installed(platform),
        "folder": platform.folder_path(),
        "exe": platform.exe_path(),
        "mode": platform.switch_mode(),
        "category": platform.category(),
    })
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------
fn platforms() -> Value {
    let mut items: Vec<Value> = book()
        .enabled()
        .into_iter()
        .map(platform_payload)
        .collect();
    // Platforms with saved accounts float to the top; installed ones next.
    items.sort_by_key(|item| {
        let accounts = item.get("accounts").and_then(Value::as_u64).unwrap_or(0);
        let installed = item.get("installed").and_then(Value::as_bool).unwrap_or(false);
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        (!(accounts > 0), !installed, name)
    });
    json!(items)
}

fn accounts(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let names = engine::list_accounts(platform);
    let current = engine::current_account(platform);
    let current_id = current
        .as_ref()
        .map(|(id, _)| id.clone())
        .unwrap_or_default();

    let mut items = Vec::with_capacity(names.len());
    for name in &names {
        let id = engine::account_id(platform, name);
        let (avatar_path, cover_path) = engine::artwork(platform, name);
        let meta = crate::localmeta::for_account(platform, name);
        let avatar = avatar_path
            .as_deref()
            .and_then(data_uri)
            .or_else(|| meta.avatar.as_deref().and_then(data_uri));
        items.push(json!({
            "name": name,
            "label": meta.name.clone().unwrap_or_else(|| name.clone()),
            "detail": meta.detail,
            "id": id,
            "current": !current_id.is_empty() && id == current_id,
            "avatar": avatar,
            "cover": cover_path.as_deref().and_then(data_uri),
            "notes": engine::notes_for(platform, name),
            "profile": crate::profile::summary(platform, name),
        }));
    }

    let settings = platform.settings();
    let prefs = load_prefs();
    Ok(json!({
        "platform": platform.name,
        "icon": platform_icon(platform),
        "accounts": items,
        "current": current.map(|(id, name)| json!({"id": id, "name": name})),
        "mode": platform.switch_mode(),
        "settings": {
            "folder": platform.folder_path(),
            "exe": platform.exe_path(),
            "installed": book().is_installed(platform),
            "autoStart": settings.get("AutoStart").and_then(Value::as_bool).unwrap_or(true),
            "clearOldLogin": prefs.get("clearOldLogin").and_then(Value::as_bool).unwrap_or(true),
        }
    }))
}

fn switch(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = account_name(params)?;
    let clear = flag(params, "clear", true);
    let start = flag(params, "start", true);
    let result = engine::switch(platform, &name, clear, start)?;
    Ok(Value::Object(result))
}

fn prepare_login(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let result = engine::prepare_login(platform)?;
    Ok(Value::Object(result))
}

fn add(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = params
        .get("name")
        .or_else(|| params.get("account"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if name.trim().is_empty() {
        return Err("No account name given.".into());
    }
    let saved = engine::add_current(platform, &name)?;
    Ok(json!({ "name": saved }))
}

fn delete(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = account_name(params)?;
    let trash = engine::delete_account(platform, &name)?;
    Ok(json!({ "deleted": name, "trash": trash }))
}

fn restore(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = account_name(params)?;
    let token = params
        .get("trash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    engine::restore_account(platform, token, &name)?;
    Ok(json!({ "restored": name }))
}

fn rename(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = account_name(params)?;
    let new_name = params
        .get("new_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if new_name.trim().is_empty() {
        return Err("The new name is empty.".into());
    }
    let saved = engine::rename_account(platform, &name, &new_name)?;
    Ok(json!({ "name": saved }))
}

fn notes(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = account_name(params)?;
    let text = params
        .get("notes")
        .and_then(Value::as_str)
        .unwrap_or_default();
    engine::set_notes(platform, &name, text)?;
    Ok(json!({ "ok": true }))
}

fn artwork_pick() -> Result<Value, String> {
    match crate::dialog::pick_image() {
        Some(path) => Ok(json!({ "path": path.display().to_string() })),
        None => Ok(json!({ "path": Value::Null })),
    }
}

fn artwork_set(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = account_name(params)?;
    let kind = params
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("avatar");
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if path.is_empty() {
        return Err("No picture was chosen.".into());
    }
    engine::set_artwork(platform, &name, kind, Path::new(path))?;
    let (avatar, cover) = engine::artwork(platform, &name);
    Ok(json!({
        "avatar": avatar.as_deref().and_then(data_uri),
        "cover": cover.as_deref().and_then(data_uri),
    }))
}

fn artwork_clear(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = account_name(params)?;
    let kind = params.get("kind").and_then(Value::as_str);
    engine::clear_artwork(platform, &name, kind)?;
    Ok(json!({ "ok": true }))
}

fn valorant_status() -> Result<Value, String> {
    Ok(crate::riot::status_json())
}

fn valorant_card(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = account_name(params)?;
    let card = crate::riot::fetch_card(platform, &name)?;
    Ok(json!({
        "liveName": card.live_name,
        "cardName": card.card_name,
        "cover": data_uri(&card.cover),
        "avatar": card.avatar.as_deref().and_then(data_uri),
    }))
}

fn riot_profile(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = account_name(params)?;
    let force = params.get("force").and_then(Value::as_bool).unwrap_or(false);
    match crate::profile::fetch(platform, &name, force)? {
        crate::profile::Outcome::Fetched(profile) => Ok(json!({
            "profile": profile,
            "summary": crate::profile::summary(platform, &name),
        })),
        crate::profile::Outcome::Mismatch { live, saved } => Ok(json!({
            "mismatch": true,
            "liveName": live,
            "savedName": saved,
        })),
    }
}

fn profile_get(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let name = account_name(params)?;
    Ok(crate::profile::cached(platform, &name).unwrap_or(Value::Null))
}

fn folder(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    if !path.is_empty() {
        let expanded = engine::expand(&path, None);
        let target = Path::new(&expanded);
        if !target.is_dir() {
            return Err(format!("Not a folder: {expanded}"));
        }
        platform.set_folder_path(&path);
    } else {
        platform.set_folder_path("");
    }

    Ok(platform_payload(platform))
}

fn auto_detect(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    if book().is_installed(platform) {
        return Ok(json!({ "folder": platform.folder_path(), "found": true }));
    }
    // Walk the usual install roots for the launcher executable name.
    let exe_name = Path::new(&platform.exe_default())
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut roots = vec![
        config::programfiles(),
        config::programfiles_x86(),
        config::localappdata(),
        config::appdata(),
    ];
    for drive in ["C:", "D:", "E:", "F:", "G:", "H:", "I:", "J:"] {
        roots.push(Path::new(drive).to_path_buf());
    }
    if exe_name.is_empty() {
        return Ok(json!({ "folder": "", "found": false }));
    }
    for root in roots {
        if let Some(found) = find_file(&root, &exe_name, 3) {
            let folder = found
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            platform.set_folder_path(&folder);
            return Ok(json!({ "folder": folder, "found": true }));
        }
    }
    Ok(json!({ "folder": "", "found": false }))
}

fn find_file(root: &Path, exe_name: &str, depth: usize) -> Option<std::path::PathBuf> {
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(root).ok()?;
    let mut dirs = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            // Skip noisy system trees.
            if matches!(
                name.as_str(),
                "windows" | "winsxs" | "system32" | "node_modules" | "$recycle.bin" | "temp"
            ) {
                continue;
            }
            if path.join(exe_name).is_file() {
                return Some(path.join(exe_name));
            }
            dirs.push(path);
        }
    }
    for dir in dirs {
        if let Some(found) = find_file(&dir, exe_name, depth - 1) {
            return Some(found);
        }
    }
    None
}

fn reveal(params: &Value) -> Result<Value, String> {
    let platform = platform(params)?;
    let target = match params.get("account").and_then(Value::as_str) {
        Some(name) if !name.is_empty() => {
            if platform.is_profile_mode() {
                let root = engine::expand(&platform.profile_root(), Some(platform));
                std::path::PathBuf::from(root).join(engine::account_id(platform, name))
            } else {
                engine::account_dir(platform, name)
            }
        }
        _ => engine::platform_dir(platform),
    };
    if !target.exists() {
        return Err("That folder does not exist yet.".into());
    }
    std::process::Command::new("explorer")
        .arg(target.display().to_string())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

fn log() -> Value {
    json!(engine::recent_log(200))
}

fn open_data() -> Result<Value, String> {
    let path = config::data_root();
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(path.display().to_string())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

fn open_log() -> Result<Value, String> {
    let path = config::log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if !path.exists() {
        let _ = std::fs::write(&path, "");
    }
    std::process::Command::new("notepad")
        .arg(path.display().to_string())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

fn quit() -> Value {
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// entry point
// ---------------------------------------------------------------------------
pub fn dispatch(method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "bootstrap" => Ok(json!({
            "platforms": platforms(),
            "prefs": prefs_payload(),
            "autostart": autostart_enabled(),
            "version": env!("CARGO_PKG_VERSION"),
            "logo": crate::assets::logo_data_uri(),
            "mark": crate::assets::mark_data_uri(),
            "dataRoot": config::data_root().display().to_string(),
            "logPath": config::log_path().display().to_string(),
        })),
        "platforms" => Ok(platforms()),
        "accounts" => accounts(params),
        "switch" => switch(params),
        "add" => add(params),
        "prepare_login" => prepare_login(params),
        "delete" => delete(params),
        "restore" => restore(params),
        "rename" => rename(params),
        "notes" => notes(params),
        "artwork_pick" => artwork_pick(),
        "artwork_set" => artwork_set(params),
        "artwork_clear" => artwork_clear(params),
        "valorant_status" => valorant_status(),
        "valorant_card" => valorant_card(params),
        "riot_profile" => riot_profile(params),
        "profile" => profile_get(params),
        "folder" => folder(params),
        "detect" => auto_detect(params),
        "reveal" => reveal(params),
        "prefs" => set_prefs(params),
        "autostart" => autostart(params),
        "log" => Ok(log()),
        "open_data" => open_data(),
        "open_log" => open_log(),
        "quit" => Ok(quit()),
        other => Err(format!("Unknown method: {other}")),
    }
}

