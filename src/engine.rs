//! The switching engine.
//!
//! Reconstructed from the shipped .NET build (`TcNo-Acc-Switcher-Server.dll`,
//! 2025.11.20.3). The contract is:
//!
//! * an account is a folder in `LoginCache/<Platform>/<Name>/`
//! * each folder mirrors the paths listed in `Platforms.json` -> `LoginFiles`
//! * `ids.json` maps the platform's unique account id -> display name
//! * `reg.json` holds registry values captured for that account
//! * switching = kill launcher -> clear live login files -> copy the account
//!   folder back over the live paths -> optionally relaunch

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};
use winreg::enums::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS};
use winreg::{HKEY, RegKey};

use crate::config;
use crate::platforms::Platform;

pub type Res<T> = Result<T, String>;

/// Hidden marker written inside a trashed session so Undo can restore its id.
const TRASH_META: &str = ".nexulon-trash.json";

// ---------------------------------------------------------------------------
// logging
// ---------------------------------------------------------------------------
static LOG: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn log_buffer() -> &'static Mutex<Vec<String>> {
    LOG.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn log(message: &str) {
    let line = format!("{} - {}", timestamp(), message);
    if let Ok(mut buffer) = log_buffer().lock() {
        buffer.push(line.clone());
        if buffer.len() > 400 {
            let drop_to = buffer.len() - 400;
            buffer.drain(..drop_to);
        }
    }
    let path = config::log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = writeln!(file, "{line}");
    }
}

pub fn recent_log(limit: usize) -> Vec<String> {
    log_buffer()
        .lock()
        .map(|buffer| {
            let start = buffer.len().saturating_sub(limit);
            buffer[start..].to_vec()
        })
        .unwrap_or_default()
}

fn timestamp() -> String {
    // Local time is not worth a dependency; UTC is fine for a log.
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("t{secs}")
}

// ---------------------------------------------------------------------------
// path expansion
// ---------------------------------------------------------------------------
pub fn expand(path: &str, platform: Option<&Platform>) -> String {
    if path.is_empty() {
        return String::new();
    }
    let mut text = path.to_string();
    if let Some(platform) = platform {
        let folder = platform.folder_path();
        let folder = if folder.is_empty() {
            // No folder chosen yet: fall back to where the default executable
            // lives, so `%Platform_Folder%` still resolves for stock installs.
            let exe = platform.exe_default();
            if exe.contains("%Platform_Folder%") {
                String::new()
            } else {
                let exe = expand(&exe, None);
                Path::new(&exe)
                    .parent()
                    .map(|parent| parent.display().to_string())
                    .unwrap_or_default()
            }
        } else {
            expand(&folder, None)
        };
        text = text.replace("%Platform_Folder%", &folder);
    }
    replace_vars(&text)
}

fn replace_vars(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            if let Some(offset) = chars[i + 1..].iter().position(|c| *c == '%') {
                let name: String = chars[i + 1..i + 1 + offset].iter().collect();
                if let Some(value) = env_value(&name) {
                    out.push_str(&value);
                    i = i + offset + 2;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn env_value(name: &str) -> Option<String> {
    let path = match name.to_ascii_uppercase().as_str() {
        "APPDATA" => config::appdata(),
        "LOCALAPPDATA" => config::localappdata(),
        "PROGRAMDATA" => config::programdata(),
        "USERPROFILE" => config::home(),
        "PROGRAMFILES" => config::programfiles(),
        "PROGRAMFILES(X86)" => config::programfiles_x86(),
        "DOCUMENTS" => config::documents(),
        "STARTMENUPROGRAMDATA" => config::start_menu_programdata(),
        _ => return std::env::var(name).ok(),
    };
    Some(path.display().to_string())
}

fn has_wildcard(path: &str) -> bool {
    path.contains('*') || path.contains('?')
}

fn dest_path(account: &Path, dest: &str) -> PathBuf {
    let cleaned = dest.replace('/', "\\");
    let trimmed = cleaned.trim_matches('\\');
    if trimmed.is_empty() {
        account.to_path_buf()
    } else {
        account.join(trimmed)
    }
}

// ---------------------------------------------------------------------------
// registry
// ---------------------------------------------------------------------------
pub fn is_reg_spec(spec: &str) -> bool {
    spec.len() >= 4 && spec[..4].eq_ignore_ascii_case("REG:")
}

fn reg_root(name: &str) -> Option<HKEY> {
    Some(match name.to_ascii_uppercase().as_str() {
        "HKCU" | "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
        "HKLM" | "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
        "HKCR" | "HKEY_CLASSES_ROOT" => HKEY_CLASSES_ROOT,
        "HKU" | "HKEY_USERS" => HKEY_USERS,
        _ => return None,
    })
}

fn reg_parts(spec: &str) -> Option<(HKEY, String, String)> {
    let body = if is_reg_spec(spec) { &spec[4..] } else { spec };
    let (root_name, rest) = body.split_once('\\')?;
    let root = reg_root(root_name)?;
    let (key_path, value_name) = rest.rsplit_once(':')?;
    if key_path.is_empty() {
        return None;
    }
    Some((root, key_path.to_string(), value_name.to_string()))
}

pub fn read_reg(spec: &str) -> Option<String> {
    let (root, key_path, value_name) = reg_parts(spec)?;
    let key = RegKey::predef(root).open_subkey(&key_path).ok()?;
    if let Ok(text) = key.get_value::<String, _>(&value_name) {
        return Some(text);
    }
    if let Ok(number) = key.get_value::<u32, _>(&value_name) {
        return Some(number.to_string());
    }
    key.get_raw_value(&value_name)
        .ok()
        .map(|raw| raw.bytes.iter().map(|b| format!("{b:02x}")).collect())
}

pub fn write_reg(spec: &str, value: &str) -> bool {
    let Some((root, key_path, value_name)) = reg_parts(spec) else {
        return false;
    };
    match RegKey::predef(root).create_subkey(&key_path) {
        Ok((key, _)) => key.set_value(&value_name, &value).is_ok(),
        Err(_) => false,
    }
}

pub fn delete_reg(spec: &str) -> bool {
    let Some((root, key_path, value_name)) = reg_parts(spec) else {
        return false;
    };
    match RegKey::predef(root).open_subkey_with_flags(&key_path, winreg::enums::KEY_SET_VALUE) {
        Ok(key) => key.delete_value(&value_name).is_ok(),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// unique account id
// ---------------------------------------------------------------------------
fn random_id(len: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        ^ (std::process::id() as u64) << 32;
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        out.push(ALPHABET[(seed % ALPHABET.len() as u64) as usize] as char);
    }
    out
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn json_select_first(document: &Value, selector: &str) -> String {
    let mut node = document;
    for part in selector
        .trim_start_matches('$')
        .trim_matches('.')
        .split('.')
        .filter(|p| !p.is_empty())
    {
        if let Value::Array(items) = node {
            match items.first() {
                Some(first) => node = first,
                None => return String::new(),
            }
        }
        match node.get(part) {
            Some(next) => node = next,
            None => return String::new(),
        }
    }
    match node {
        Value::Array(items) => items.first().map(value_to_string).unwrap_or_default(),
        other => value_to_string(other),
    }
}

pub fn read_unique_id(platform: &Platform, create: bool) -> String {
    let method = platform.unique_id_method().to_ascii_uppercase();
    let spec = platform.unique_id_file();

    if method.starts_with("CREATE_ID_FILE") {
        let path = PathBuf::from(expand(&spec, Some(platform)));
        if let Ok(text) = fs::read_to_string(&path) {
            return text.trim().to_string();
        }
        if create {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let value = random_id(16);
            let _ = fs::write(&path, &value);
            return value;
        }
        return String::new();
    }

    if method.starts_with("JSON_SELECT_FIRST") {
        let (file_part, selector) = match spec.split_once("::") {
            Some((file, selector)) => (file, selector),
            None => (spec.as_str(), "$"),
        };
        let path = PathBuf::from(expand(file_part, Some(platform)));
        let Ok(text) = fs::read_to_string(&path) else {
            return String::new();
        };
        let Ok(document) = serde_json::from_str::<Value>(&text) else {
            return String::new();
        };
        return json_select_first(&document, selector);
    }

    if method == "REGKEY" || is_reg_spec(&spec) {
        let key = if is_reg_spec(&spec) {
            spec.clone()
        } else {
            format!("REG:{spec}")
        };
        return read_reg(&key).unwrap_or_default();
    }

    // Default: read the file and apply UniqueIdRegex (or take the whole content).
    let path = PathBuf::from(expand(&spec, Some(platform)));
    let Ok(text) = fs::read_to_string(&path) else {
        return String::new();
    };
    let regex = platform.unique_id_regex();
    if regex.is_empty() {
        return text.trim().to_string();
    }
    if regex == "EMAIL_REGEX" {
        return find_email(&text).unwrap_or_default();
    }
    if regex == "WIN_FILEPATH_REGEX" {
        return String::new();
    }
    match regex::Regex::new(&regex) {
        // A capture group, when the expression has one, is the id itself; the
        // whole match is only used for simple patterns.
        Ok(re) => re
            .captures(&text)
            .and_then(|captures| captures.get(1).or_else(|| captures.get(0)))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default(),
        Err(_) => String::new(),
    }
}

fn find_email(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"[\w.+-]+@[\w-]+\.[\w.]+").ok()?;
    re.find(text).map(|m| m.as_str().to_string())
}

// ---------------------------------------------------------------------------
// account cache
// ---------------------------------------------------------------------------
pub fn platform_dir(platform: &Platform) -> PathBuf {
    config::login_cache().join(&platform.name)
}

pub fn account_dir(platform: &Platform, name: &str) -> PathBuf {
    platform_dir(platform).join(name)
}

fn ids_path(platform: &Platform) -> PathBuf {
    platform_dir(platform).join("ids.json")
}

fn order_path(platform: &Platform) -> PathBuf {
    platform_dir(platform).join("order.json")
}

pub fn read_ids(platform: &Platform) -> Map<String, Value> {
    fs::read_to_string(ids_path(platform))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default()
}

pub fn write_ids(platform: &Platform, ids: &Map<String, Value>) {
    let path = ids_path(platform);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let text = serde_json::to_string(&Value::Object(ids.clone())).unwrap_or_else(|_| "{}".into());
    let _ = fs::write(path, text);
}

fn read_order(platform: &Platform) -> Vec<String> {
    fs::read_to_string(order_path(platform))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| match value {
            Value::Array(items) => Some(
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

pub fn write_order(platform: &Platform, order: &[String]) {
    let path = order_path(platform);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, serde_json::to_string(order).unwrap_or_else(|_| "[]".into()));
}

pub fn account_id(platform: &Platform, name: &str) -> String {
    if platform.is_profile_mode() {
        return profile_accounts(platform)
            .into_iter()
            .find(|(display, _)| display == name)
            .map(|(_, folder)| folder)
            .unwrap_or_default();
    }
    let instance = account_dir(platform, name).join(config::ID_FILE);
    if let Ok(text) = fs::read_to_string(&instance) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    read_ids(platform)
        .into_iter()
        .find(|(_, value)| value.as_str() == Some(name))
        .map(|(id, _)| id)
        .unwrap_or_default()
}

pub fn list_accounts(platform: &Platform) -> Vec<String> {
    if platform.is_profile_mode() {
        return profile_accounts(platform)
            .into_iter()
            .map(|(display, _)| display)
            .collect();
    }
    let folder = platform_dir(platform);
    let Ok(entries) = fs::read_dir(&folder) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| !config::is_non_account(name) && !name.starts_with('.'))
        .collect();

    let order = read_order(platform);
    names.sort_by_key(|name| {
        let rank = order.iter().position(|n| n == name).unwrap_or(usize::MAX);
        (rank, name.to_ascii_lowercase())
    });
    names
}

// ---------------------------------------------------------------------------
// profile-mode platforms (browsers)
// ---------------------------------------------------------------------------
/// Profiles a `profile`-mode platform can launch, as `(display name, folder)`.
pub fn profile_accounts(platform: &Platform) -> Vec<(String, String)> {
    let root = PathBuf::from(expand(&platform.profile_root(), Some(platform)));
    if !root.is_dir() {
        return Vec::new();
    }
    let mut items = match platform.profile_list().to_ascii_lowercase().as_str() {
        "chrome" => chrome_profiles(&root),
        "firefox" => firefox_profiles(&root),
        _ => dirs_profiles(&root),
    };
    items.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()));
    items
}

fn dirs_profiles(root: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| !name.starts_with('.'))
        .map(|name| (name.clone(), name))
        .collect()
}

fn chrome_profiles(root: &Path) -> Vec<(String, String)> {
    let mut items: Vec<(String, String)> = Vec::new();
    // `Local State` lists every profile and carries its display name.
    if let Ok(text) = fs::read_to_string(root.join("Local State")) {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            if let Some(cache) = value
                .get("profile")
                .and_then(|profile| profile.get("info_cache"))
                .and_then(Value::as_object)
            {
                for (folder, body) in cache {
                    if !root.join(folder).is_dir() {
                        continue;
                    }
                    let name = body
                        .get("name")
                        .and_then(Value::as_str)
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or(folder);
                    items.push((name.to_string(), folder.clone()));
                }
            }
        }
    }
    if items.is_empty() {
        items = dirs_profiles(root)
            .into_iter()
            .filter(|(folder, _)| folder == "Default" || folder.starts_with("Profile "))
            .collect();
    }
    items
}

fn firefox_profiles(root: &Path) -> Vec<(String, String)> {
    // `profiles.ini` sits one level above the `Profiles` folder.
    let ini = root
        .parent()
        .map(|parent| parent.join("profiles.ini"))
        .unwrap_or_else(|| root.join("profiles.ini"));
    let mut items: Vec<(String, String)> = Vec::new();
    if let Ok(text) = fs::read_to_string(&ini) {
        let mut name = String::new();
        let mut path = String::new();
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                push_firefox_profile(&mut items, root, &name, &path);
                name.clear();
                path.clear();
            } else if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "Name" => name = value.trim().to_string(),
                    "Path" => path = value.trim().to_string(),
                    _ => {}
                }
            }
        }
        push_firefox_profile(&mut items, root, &name, &path);
    }
    if items.is_empty() {
        items = dirs_profiles(root);
    }
    items
}

fn push_firefox_profile(items: &mut Vec<(String, String)>, root: &Path, name: &str, path: &str) {
    if name.is_empty() || path.is_empty() {
        return;
    }
    let folder = Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    if folder.is_empty() || !root.join(&folder).is_dir() {
        return;
    }
    if items.iter().any(|(_, existing)| existing == &folder) {
        return;
    }
    items.push((name.to_string(), folder));
}

/// Split the argument template before substituting, so a profile path with
/// spaces still becomes one argument.
fn profile_args(template: &str, folder: &str, path: &str) -> Vec<String> {
    template
        .split_whitespace()
        .map(|piece| piece.replace("{folder}", folder).replace("{path}", path))
        .collect()
}

fn launch_profile(platform: &Platform, folder: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    let exe = platform.exe_path();
    if exe.is_empty() || !Path::new(&exe).is_file() {
        log(&format!("launcher not found for {}: {exe}", platform.name));
        return None;
    }
    let root = PathBuf::from(expand(&platform.profile_root(), Some(platform)));
    let path = root.join(folder);
    let mut command = Command::new(&exe);
    for arg in profile_args(&platform.profile_arg(), folder, &path.display().to_string()) {
        command.arg(arg);
    }
    for arg in platform.exe_extra_args().split_whitespace() {
        command.arg(arg);
    }
    if let Some(parent) = Path::new(&exe).parent() {
        command.current_dir(parent);
    }
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    match command.spawn() {
        Ok(_) => Some(exe),
        Err(error) => {
            log(&format!("failed to launch {exe}: {error}"));
            None
        }
    }
}

fn switch_profile(platform: &Platform, name: &str, start: bool) -> Res<Map<String, Value>> {
    let Some((display, folder)) = profile_accounts(platform)
        .into_iter()
        .find(|(display, _)| display == name)
    else {
        return Err(format!("No {name:?} profile was found for {}.", platform.name));
    };

    let settings = platform.settings();
    let method = settings
        .get("ClosingMethod")
        .and_then(Value::as_str)
        .unwrap_or("Combined")
        .to_string();
    let auto_start = settings
        .get("AutoStart")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    kill_processes(platform, &method);
    let started = start && auto_start && launch_profile(platform, &folder).is_some();

    let mut settings = platform.settings();
    settings.insert("LastAccount".into(), json!(display));
    platform.save_settings(&settings);

    log(&format!(
        "opened {} profile {} (started={started})",
        platform.name, display
    ));

    let mut result = Map::new();
    result.insert("platform".into(), json!(platform.name));
    result.insert("account".into(), json!(display));
    result.insert("id".into(), json!(folder));
    result.insert("cleared".into(), json!([]));
    result.insert("missing".into(), json!([]));
    result.insert("started".into(), json!(started));
    Ok(result)
}

// ---------------------------------------------------------------------------
// per-account artwork
// ---------------------------------------------------------------------------
/// Stable file key for an account name (FNV-1a), so any name is filesystem
/// safe and renaming never collides with another account.
fn art_key(name: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn artwork_index_path(platform: &Platform) -> PathBuf {
    config::artwork_dir().join(&platform.name).join("index.json")
}

fn read_artwork(platform: &Platform) -> Map<String, Value> {
    fs::read_to_string(artwork_index_path(platform))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default()
}

fn write_artwork(platform: &Platform, index: &Map<String, Value>) {
    let path = artwork_index_path(platform);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let text = serde_json::to_string(&Value::Object(index.clone())).unwrap_or_else(|_| "{}".into());
    let _ = fs::write(path, text);
}

/// `(avatar, cover)` for an account, when the user set them.
pub fn artwork(platform: &Platform, name: &str) -> (Option<PathBuf>, Option<PathBuf>) {
    let folder = config::artwork_dir().join(&platform.name);
    let index = read_artwork(platform);
    let entry = index.get(name).and_then(Value::as_object);
    let pick = |key: &str| {
        entry
            .and_then(|entry| entry.get(key))
            .and_then(Value::as_str)
            .map(|file| folder.join(file))
            .filter(|path| path.is_file())
    };
    (pick("avatar"), pick("cover"))
}

pub fn set_artwork(platform: &Platform, name: &str, kind: &str, source: &Path) -> Res<()> {
    if !source.is_file() {
        return Err("That picture no longer exists.".into());
    }
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let bytes = fs::read(source).map_err(|error| error.to_string())?;
    set_artwork_bytes(platform, name, kind, &bytes, &extension).map(|_| ())
}

/// Store artwork that was downloaded instead of picked, e.g. a Valorant card.
pub fn set_artwork_bytes(
    platform: &Platform,
    name: &str,
    kind: &str,
    bytes: &[u8],
    extension: &str,
) -> Res<PathBuf> {
    let kind = match kind {
        "avatar" => "avatar",
        "cover" => "cover",
        _ => return Err("Unknown picture kind.".into()),
    };
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp") {
        return Err("Pick a PNG, JPG, WEBP, GIF or BMP image.".into());
    }
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("That picture is larger than 16 MB.".into());
    }

    let folder = config::artwork_dir().join(&platform.name);
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let file = format!("{}-{}.{}", kind, art_key(name), extension);
    let target = folder.join(&file);
    fs::write(&target, bytes).map_err(|error| error.to_string())?;

    let mut index = read_artwork(platform);
    let mut entry = index
        .get(name)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if let Some(old) = entry.get(kind).and_then(Value::as_str) {
        if old != file {
            let _ = fs::remove_file(folder.join(old));
        }
    }
    entry.insert(kind.into(), json!(file));
    index.insert(name.to_string(), Value::Object(entry));
    write_artwork(platform, &index);
    log(&format!("set {kind} for {}/{name}", platform.name));
    Ok(target)
}

pub fn clear_artwork(platform: &Platform, name: &str, kind: Option<&str>) -> Res<()> {
    let folder = config::artwork_dir().join(&platform.name);
    let mut index = read_artwork(platform);
    let Some(mut entry) = index.get(name).and_then(Value::as_object).cloned() else {
        return Ok(());
    };
    let kinds: Vec<&str> = match kind {
        Some(kind) if !kind.is_empty() => vec![kind],
        _ => vec!["avatar", "cover"],
    };
    for key in kinds {
        if let Some(old) = entry.remove(key).and_then(|value| value.as_str().map(str::to_string)) {
            let _ = fs::remove_file(folder.join(old));
        }
    }
    if entry.is_empty() {
        index.remove(name);
    } else {
        index.insert(name.to_string(), Value::Object(entry));
    }
    write_artwork(platform, &index);
    Ok(())
}

pub fn notes_for(platform: &Platform, name: &str) -> String {
    let id = account_id(platform, name);
    platform
        .settings()
        .get("AccountNotes")
        .and_then(Value::as_object)
        .and_then(|notes| notes.get(&id).or_else(|| notes.get(name)))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn set_notes(platform: &Platform, name: &str, text: &str) -> Res<()> {
    let mut settings = platform.settings();
    let mut notes = settings
        .get("AccountNotes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let key = {
        let id = account_id(platform, name);
        if id.is_empty() { name.to_string() } else { id }
    };
    if text.trim().is_empty() {
        notes.remove(&key);
    } else {
        notes.insert(key, json!(text));
    }
    settings.insert("AccountNotes".into(), Value::Object(notes));
    platform.save_settings(&settings);
    Ok(())
}

pub fn clean_name(name: &str) -> String {
    let mut cleaned = String::with_capacity(name.len());
    for ch in name.trim().chars() {
        match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => cleaned.push('-'),
            _ => cleaned.push(ch),
        }
    }
    let cleaned = cleaned.trim_end_matches(['.', ' ']).to_string();
    if cleaned.is_empty() {
        "Account".to_string()
    } else {
        cleaned
    }
}

// ---------------------------------------------------------------------------
// copy engine
// ---------------------------------------------------------------------------
fn copy_tree_into(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree_into(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn copy_file(src: &Path, dest: &Path) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dest)?;
    Ok(())
}

fn glob_matches(pattern: &str) -> Vec<PathBuf> {
    glob::glob(pattern)
        .map(|paths| paths.filter_map(Result::ok).collect())
        .unwrap_or_default()
}

pub fn copy_out(platform: &Platform, target: &Path) -> Vec<String> {
    let _ = fs::create_dir_all(target);
    let mut reg_values = Map::new();
    let mut missing = Vec::new();

    for (src, dest) in platform.login_files() {
        if is_reg_spec(&src) {
            match read_reg(&src) {
                Some(value) => {
                    reg_values.insert(src, json!(value));
                }
                None => missing.push(src),
            }
            continue;
        }

        let live = expand(&src, Some(platform));
        let dest_dir = dest_path(target, &dest);

        if has_wildcard(&live) {
            let matches = glob_matches(&live);
            if matches.is_empty() {
                missing.push(src);
                continue;
            }
            let _ = fs::create_dir_all(&dest_dir);
            for found in matches {
                let name = found
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("item"));
                let to = dest_dir.join(name);
                if found.is_dir() {
                    let _ = copy_tree_into(&found, &to);
                } else {
                    let _ = copy_file(&found, &to);
                }
            }
            continue;
        }

        let source = Path::new(&live);
        if source.is_dir() {
            let _ = copy_tree_into(source, &dest_dir);
        } else if source.is_file() {
            if dest.replace('/', "\\").ends_with('\\') {
                let name = source
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("file"));
                let _ = copy_file(source, &dest_dir.join(name));
            } else {
                let _ = copy_file(source, &dest_dir);
            }
        } else {
            missing.push(src);
        }
    }

    if !reg_values.is_empty() {
        let text = serde_json::to_string_pretty(&Value::Object(reg_values)).unwrap_or_default();
        let _ = fs::write(target.join(config::REG_FILE), text);
    }
    missing
}

pub fn copy_in(platform: &Platform, source: &Path) -> Vec<String> {
    let mut missing = Vec::new();

    let reg_file = source.join(config::REG_FILE);
    if let Ok(text) = fs::read_to_string(&reg_file) {
        if let Ok(Value::Object(values)) = serde_json::from_str::<Value>(&text) {
            for (spec, value) in values {
                write_reg(&spec, &value_to_string(&value));
            }
        }
    }

    for (src, dest) in platform.login_files() {
        if is_reg_spec(&src) {
            continue;
        }
        let stored = dest_path(source, &dest);
        if !stored.exists() {
            missing.push(dest.clone());
            continue;
        }

        let live = expand(&src, Some(platform));

        if has_wildcard(&live) {
            let parent = Path::new(&live)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            let _ = fs::create_dir_all(&parent);
            if stored.is_dir() {
                if let Ok(entries) = fs::read_dir(&stored) {
                    for entry in entries.filter_map(Result::ok) {
                        let to = parent.join(entry.file_name());
                        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            let _ = copy_tree_into(&entry.path(), &to);
                        } else {
                            let _ = copy_file(&entry.path(), &to);
                        }
                    }
                }
            } else {
                let name = stored
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("file"));
                let _ = copy_file(&stored, &parent.join(name));
            }
            continue;
        }

        if stored.is_dir() {
            let _ = copy_tree_into(&stored, Path::new(&live));
        } else {
            let _ = copy_file(&stored, Path::new(&live));
        }
    }

    missing
}

fn clear_targets(platform: &Platform) -> Vec<String> {
    let mut targets: Vec<String> = Vec::new();
    for entry in platform.path_list_to_clear() {
        if entry.eq_ignore_ascii_case("SAME_AS_LOGIN_FILES") {
            for (src, _) in platform.login_files() {
                if !is_reg_spec(&src) {
                    targets.push(src);
                }
            }
        } else if !is_reg_spec(&entry) {
            targets.push(entry);
        }
    }
    let mut unique: Vec<String> = Vec::new();
    for target in targets {
        if !unique.contains(&target) {
            unique.push(target);
        }
    }
    unique
}

fn remove_path(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

pub fn clear_login_files(platform: &Platform) -> Vec<String> {
    let mut cleared = Vec::new();

    for src in clear_targets(platform) {
        let live = expand(&src, Some(platform));
        if has_wildcard(&live) {
            for found in glob_matches(&live) {
                remove_path(&found);
                cleared.push(found.display().to_string());
            }
            continue;
        }
        let path = Path::new(&live);
        if path.exists() {
            remove_path(path);
            cleared.push(live.clone());
        }
    }

    if platform.reg_delete_on_clear() {
        for (src, _) in platform.login_files() {
            if is_reg_spec(&src) && delete_reg(&src) {
                cleared.push(src);
            }
        }
    }

    for cache in platform.cache_paths() {
        let live = expand(&cache, Some(platform));
        if has_wildcard(&live) {
            for found in glob_matches(&live) {
                remove_path(&found);
            }
        } else {
            let path = Path::new(&live);
            if path.exists() {
                remove_path(path);
            }
        }
    }

    cleared
}

// ---------------------------------------------------------------------------
// processes
// ---------------------------------------------------------------------------
fn taskkill(exe: &str, force: bool) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new("taskkill");
    if force {
        command.arg("/F");
    }
    command.args(["/IM", exe]).creation_flags(CREATE_NO_WINDOW);
    let _ = command.output();
}

pub fn kill_processes(platform: &Platform, method: &str) -> Vec<String> {
    let force_only = method.eq_ignore_ascii_case("taskkill");
    for exe in platform.exes_to_end() {
        if force_only {
            taskkill(&exe, true);
        } else {
            taskkill(&exe, false);
            taskkill(&exe, true);
        }
    }
    platform.exes_to_end()
}

pub fn launch(platform: &Platform) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    let exe = platform.exe_path();
    if exe.is_empty() || !Path::new(&exe).is_file() {
        log(&format!("launcher not found for {}: {exe}", platform.name));
        return None;
    }
    let mut command = Command::new(&exe);
    for arg in platform.exe_extra_args().split_whitespace() {
        command.arg(arg);
    }
    if let Some(parent) = Path::new(&exe).parent() {
        command.current_dir(parent);
    }
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    match command.spawn() {
        Ok(_) => Some(exe),
        Err(error) => {
            log(&format!("failed to launch {exe}: {error}"));
            None
        }
    }
}

// ---------------------------------------------------------------------------
// high level operations
// ---------------------------------------------------------------------------
/// Close the launcher, wipe the live sign-in and start it again, so the user
/// can sign in with a *different* account before that session is captured.
///
/// Adding an account without this only snapshots whoever is already signed in,
/// which is why "add" used to save duplicates of the current account.
pub fn prepare_login(platform: &Platform) -> Res<Map<String, Value>> {
    if platform.is_profile_mode() {
        return Err(format!(
            "{} switches profiles, so there is no login data to clear.",
            platform.name
        ));
    }
    let exe = platform.exe_path();
    if exe.is_empty() || !Path::new(&exe).is_file() {
        return Err(format!(
            "The launcher for {} was not found, so the current sign-in was left alone. Set its location first.",
            platform.name
        ));
    }
    let method = platform
        .settings()
        .get("ClosingMethod")
        .and_then(Value::as_str)
        .unwrap_or("Combined")
        .to_string();

    kill_processes(platform, &method);
    let cleared = clear_login_files(platform);
    let launched = launch(platform);

    log(&format!(
        "prepared a fresh sign-in for {} (cleared={}, launched={})",
        platform.name,
        cleared.len(),
        launched.is_some()
    ));

    let mut result = Map::new();
    result.insert("platform".into(), json!(platform.name));
    result.insert("cleared".into(), json!(cleared.len()));
    result.insert("launched".into(), json!(launched));
    Ok(result)
}

/// Riot keeps its refresh token in the live YAML only while the client is
/// signed in. Capturing before that point would store an empty session and
/// produce an account that duplicates whatever was there before.
fn sign_in_problem(platform: &Platform) -> Option<String> {
    if !platform.name.to_ascii_lowercase().contains("riot") {
        return None;
    }
    match crate::riot::signed_in() {
        Ok((name, _)) if !name.trim().is_empty() => None,
        _ => Some(
            "The Riot Client is not signed in yet, so there is no account to save. Sign in to the Riot Client, then press Add account."
                .to_string(),
        ),
    }
}

pub fn add_current(platform: &Platform, name: &str) -> Res<String> {
    if platform.is_profile_mode() {
        return Err(format!(
            "{} switches profiles, so there is no login data to capture. Its profiles are listed automatically.",
            platform.name
        ));
    }
    let name = clean_name(name);
    let target = account_dir(platform, &name);
    if target.exists() {
        return Err(format!("An account named {name:?} already exists."));
    }

    if let Some(problem) = sign_in_problem(platform) {
        return Err(problem);
    }

    let uid = read_unique_id(platform, true);
    if !uid.is_empty() {
        if let Some(existing) = read_ids(platform).get(&uid).and_then(Value::as_str) {
            if existing != name {
                return Err(format!(
                    "This session is already saved as {existing:?}. Close this dialog and open Add account again to start a fresh sign-in."
                ));
            }
        }
    }
    let missing = copy_out(platform, &target);

    if platform.all_files_required() && !missing.is_empty() {
        let _ = fs::remove_dir_all(&target);
        return Err(format!(
            "Not all required files were found. Is the launcher signed in? Missing: {}",
            missing.join(", ")
        ));
    }

    let empty = fs::read_dir(&target)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(true);
    if empty {
        let _ = fs::remove_dir_all(&target);
        return Err("Nothing to save: the launcher has no login data on disk.".into());
    }

    if !uid.is_empty() {
        let mut ids = read_ids(platform);
        ids.insert(uid.clone(), json!(name));
        write_ids(platform, &ids);
    }
    log(&format!(
        "added {}/{} (id={})",
        platform.name,
        name,
        if uid.is_empty() { "unknown" } else { &uid }
    ));
    Ok(name)
}

pub fn switch(
    platform: &Platform,
    name: &str,
    clear: bool,
    start: bool,
) -> Res<Map<String, Value>> {
    if platform.is_profile_mode() {
        return switch_profile(platform, name, start);
    }
    let source = account_dir(platform, name);
    if !source.is_dir() {
        return Err(format!("Account folder not found: {}", source.display()));
    }

    let settings = platform.settings();
    let method = settings
        .get("ClosingMethod")
        .and_then(Value::as_str)
        .unwrap_or("Combined")
        .to_string();
    let auto_start = settings
        .get("AutoStart")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    kill_processes(platform, &method);
    let cleared = if clear {
        clear_login_files(platform)
    } else {
        Vec::new()
    };
    let missing = copy_in(platform, &source);

    let started = start && auto_start && launch(platform).is_some();
    let uid = account_id(platform, name);

    log(&format!(
        "switched {} -> {} (id={}, started={started})",
        platform.name,
        name,
        if uid.is_empty() { "unknown" } else { &uid }
    ));

    let mut result = Map::new();
    result.insert("platform".into(), json!(platform.name));
    result.insert("account".into(), json!(name));
    result.insert("id".into(), json!(uid));
    result.insert("cleared".into(), json!(cleared));
    result.insert("missing".into(), json!(missing));
    result.insert("started".into(), json!(started));
    Ok(result)
}

pub fn delete_account(platform: &Platform, name: &str) -> Res<String> {
    if platform.is_profile_mode() {
        return Err(format!(
            "{} profiles belong to the app itself; remove them there, not here.",
            platform.name
        ));
    }
    let folder = account_dir(platform, name);
    if !folder.is_dir() {
        return Err(format!("Account folder not found: {}", folder.display()));
    }

    let uid = account_id(platform, name);

    // Move the session into a hidden trash folder instead of erasing it. The
    // interface offers an Undo action, and a stray click can never destroy a
    // saved login outright.
    let trash = platform_dir(platform).join(".trash");
    fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let mut token = format!("{name}.{stamp}");
    let mut suffix = 1;
    while trash.join(&token).exists() {
        token = format!("{name}.{stamp}.{suffix}");
        suffix += 1;
    }
    let entry = trash.join(&token);
    fs::rename(&folder, &entry).map_err(|e| e.to_string())?;

    // Platforms differ in which file holds the account id, so capture it now
    // instead of guessing on restore.
    let meta = json!({ "name": name, "id": uid });
    let _ = fs::write(
        entry.join(TRASH_META),
        serde_json::to_string(&meta).unwrap_or_default(),
    );

    if !uid.is_empty() {
        let mut ids = read_ids(platform);
        ids.remove(&uid);
        write_ids(platform, &ids);
    }
    let order: Vec<String> = read_order(platform)
        .into_iter()
        .filter(|entry| entry != name)
        .collect();
    write_order(platform, &order);
    log(&format!("trashed {}/{} -> {}", platform.name, name, token));
    Ok(token)
}

/// Put a trashed session back under its original name.
pub fn restore_account(platform: &Platform, token: &str, name: &str) -> Res<()> {
    if token.is_empty() || token.contains('/') || token.contains('\\') || token.contains("..") {
        return Err("That trash entry is not valid.".into());
    }
    let source = platform_dir(platform).join(".trash").join(token);
    if !source.is_dir() {
        return Err("That deleted account is no longer in the trash.".into());
    }
    let target = account_dir(platform, name);
    if target.exists() {
        return Err(format!("An account named {name:?} already exists."));
    }
    let meta = fs::read_to_string(source.join(TRASH_META))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let saved_id = meta
        .as_ref()
        .and_then(|value| value.get("id"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    fs::rename(&source, &target).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(target.join(TRASH_META));

    let uid = if saved_id.is_empty() {
        account_id(platform, name)
    } else {
        saved_id
    };
    if !uid.is_empty() {
        let mut ids = read_ids(platform);
        ids.insert(uid, json!(name));
        write_ids(platform, &ids);
    }
    log(&format!("restored {}/{}", platform.name, name));
    Ok(())
}

pub fn rename_account(platform: &Platform, name: &str, new_name: &str) -> Res<String> {
    if platform.is_profile_mode() {
        return Err("Profile names come from the app itself and cannot be renamed here.".into());
    }
    let new_name = clean_name(new_name);
    let folder = account_dir(platform, name);
    let target = account_dir(platform, &new_name);
    if !folder.is_dir() {
        return Err(format!("Account folder not found: {}", folder.display()));
    }
    if target.exists() {
        return Err(format!("An account named {new_name:?} already exists."));
    }
    // Read the id before the rename: the ids.json lookup keys off the old name.
    let uid = account_id(platform, name);
    fs::rename(&folder, &target).map_err(|e| e.to_string())?;
    if !uid.is_empty() {
        let mut ids = read_ids(platform);
        ids.insert(uid, json!(new_name));
        write_ids(platform, &ids);
    }

    // Pictures travel with the account: re-key both the index and the files.
    let art_folder = config::artwork_dir().join(&platform.name);
    let mut art = read_artwork(platform);
    if let Some(mut entry) = art.remove(name).and_then(|value| value.as_object().cloned()) {
        for kind in ["avatar", "cover"] {
            if let Some(old) = entry.get(kind).and_then(Value::as_str).map(str::to_string) {
                let extension = Path::new(&old)
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("png")
                    .to_string();
                let file = format!("{kind}-{}.{extension}", art_key(&new_name));
                if fs::rename(art_folder.join(&old), art_folder.join(&file)).is_ok() {
                    entry.insert(kind.into(), json!(file));
                }
            }
        }
        art.insert(new_name.clone(), Value::Object(entry));
        write_artwork(platform, &art);
    }

    crate::profile::rekey(platform, name, &new_name);

    log(&format!(
        "renamed {}/{} -> {}",
        platform.name, name, new_name
    ));
    Ok(new_name)
}

pub fn current_account(platform: &Platform) -> Option<(String, String)> {
    if platform.is_profile_mode() {
        let last = platform
            .settings()
            .get("LastAccount")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if last.is_empty() {
            return None;
        }
        let id = account_id(platform, &last);
        return Some((id, last));
    }
    let uid = read_unique_id(platform, false);
    if uid.is_empty() {
        return None;
    }
    let mut name = read_ids(platform)
        .into_iter()
        .find(|(_, value)| value.as_str() == Some(uid.as_str()))
        .and_then(|(_, value)| value.as_str().map(str::to_string))
        .unwrap_or_default();
    if name.is_empty() {
        name = list_accounts(platform)
            .into_iter()
            .find(|candidate| account_id(platform, candidate) == uid)
            .unwrap_or_default();
    }
    Some((uid, name))
}

/// Platform artwork ships inside the executable, so it is always available.
pub fn platform_icon(platform: &Platform) -> Option<String> {
    crate::assets::platform_svg(&platform.name)
}
