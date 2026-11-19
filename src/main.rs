//! Nexulon Switcher — a compact account switcher for Windows game launchers.
//!
//! The window is a native tao window hosting a WebView2 surface. The page is
//! compiled into the executable, so the portable build is one file and there is
//! no local web server. Requests travel over the WebView2 IPC channel and are
//! executed on a worker thread so the interface never blocks during a switch.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod bridge;
mod config;
mod dialog;
mod engine;
mod http;
mod localmeta;
mod platforms;
mod profile;
mod riot;

use std::sync::Mutex;

use serde_json::{Value, json};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Icon, WindowBuilder},
};
use wry::WebViewBuilder;
const WINDOW_ICON: &[u8] = include_bytes!("../assets/icon.rgba");
const ICON_SIZE: u32 = 256;

const PAGE_HTML: &str = include_str!("../web/index.html");
const PAGE_CSS: &str = include_str!("../web/style.css");
const PAGE_JS: &str = include_str!("../web/app.js");

/// The engine touches launcher folders and process lists. Requests are queued
/// through one lock so two switches can never interleave.
static ENGINE_LOCK: Mutex<()> = Mutex::new(());

enum UserEvent {
    RunScript(String),
    Theme(String, String, String),
}

fn page() -> String {
    // Build the document by hand: `str::replace` would interpret `$` sequences
    // inside the CSS/JS payloads, and the script legitimately contains them.
    let mut out = String::with_capacity(PAGE_HTML.len() + PAGE_CSS.len() + PAGE_JS.len());
    let mut rest = PAGE_HTML;
    for (marker, payload) in [
        ("/*__NEXULON_CSS__*/", PAGE_CSS),
        ("/*__NEXULON_JS__*/", PAGE_JS),
    ] {
        match rest.find(marker) {
            Some(index) => {
                out.push_str(&rest[..index]);
                out.push_str(payload);
                rest = &rest[index + marker.len()..];
            }
            None => return PAGE_HTML.to_string(),
        }
    }
    out.push_str(rest);
    out
}

/// Turn one IPC message into the JavaScript that resolves the page-side promise.
fn answer(body: &str) -> String {
    let Ok(message) = serde_json::from_str::<Value>(body) else {
        return String::new();
    };
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    let outcome = std::panic::catch_unwind(|| bridge::dispatch(&method, &params));
    let (ok, payload) = match outcome {
        Ok(Ok(value)) => (true, value),
        Ok(Err(message)) => (false, json!(message)),
        Err(_) => (false, json!("The switcher hit an internal error.")),
    };
    let literal = serde_json::to_string(&payload).unwrap_or_else(|_| "null".into());
    format!("window.__nexulonReply({id}, {ok}, {literal});")
}

#[cfg(windows)]
#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: isize,
        attribute: u32,
        value: *const core::ffi::c_void,
        size: u32,
    ) -> i32;
}

/// Turn `#rrggbb` into a Win32 `COLORREF` (`0x00BBGGRR`).
#[cfg(windows)]
fn color_ref(hex: &str) -> Option<u32> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() < 6 {
        return None;
    }
    let red = u32::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u32::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u32::from_str_radix(&hex[4..6], 16).ok()?;
    Some(red | (green << 8) | (blue << 16))
}

/// Paint the native caption in the page's colours so the title bar and the
/// top of the app read as one surface instead of two stacked themes.
#[cfg(windows)]
fn apply_title_theme(window: &tao::window::Window, background: &str, text: &str, border: &str) {
    use tao::platform::windows::WindowExtWindows;

    let Some(background) = color_ref(background) else {
        return;
    };
    let text = color_ref(text).unwrap_or(0x00ff_ffff);
    let border = color_ref(border).unwrap_or(background);
    let hwnd = window.hwnd();
    let dark: i32 = 1;
    let rounded: i32 = 2;
    unsafe {
        // 19 is the pre-20H1 attribute, 20 is the current one. Setting both
        // is harmless and keeps the caption dark on either Windows build.
        let _ = DwmSetWindowAttribute(hwnd, 19, &dark as *const _ as *const _, 4);
        let _ = DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as *const _, 4);
        let _ = DwmSetWindowAttribute(hwnd, 33, &rounded as *const _ as *const _, 4);
        let _ = DwmSetWindowAttribute(hwnd, 34, &border as *const _ as *const _, 4);
        let _ = DwmSetWindowAttribute(hwnd, 35, &background as *const _ as *const _, 4);
        let _ = DwmSetWindowAttribute(hwnd, 36, &text as *const _ as *const _, 4);
    }
}

#[cfg(not(windows))]
fn apply_title_theme(_: &tao::window::Window, _: &str, _: &str, _: &str) {}

/// `--dump` answers one bridge method on stdout and exits. It is a development
/// aid, so release builds ignore it entirely and have no command-line surface.
#[cfg(debug_assertions)]
fn dump(args: &[String]) -> Option<i32> {
    let position = args.iter().position(|argument| argument == "--dump")?;
    let method = args
        .get(position + 1)
        .map(String::as_str)
        .unwrap_or("bootstrap");
    let params = match args.get(position + 2) {
        Some(raw) if raw.trim_start().starts_with('{') => {
            serde_json::from_str::<Value>(raw).unwrap_or(Value::Null)
        }
        Some(platform) => json!({ "platform": platform }),
        None => Value::Null,
    };
    match bridge::dispatch(method, &params) {
        Ok(data) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
            Some(0)
        }
        Err(message) => {
            eprintln!("error: {message}");
            Some(1)
        }
    }
}

/// Release builds never answer `--dump`.
#[cfg(not(debug_assertions))]
fn dump(_: &[String]) -> Option<i32> {
    None
}

/// The page is compiled into the executable and never navigates. The first
/// navigation is the document WebView2 builds from that HTML; every later one
/// must be byte-identical to it, so a stray link, a dropped file or injected
/// markup cannot pull a remote page into the privileged webview.
struct NavigationGuard(Mutex<Option<String>>);

impl NavigationGuard {
    fn new() -> Self {
        Self(Mutex::new(None))
    }

    fn allow(&self, url: String) -> bool {
        let mut first = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match first.as_deref() {
            None => {
                *first = Some(url);
                true
            }
            Some(allowed) => {
                if allowed == url {
                    true
                } else {
                    let shown: String = url.chars().take(120).collect();
                    engine::log(&format!("blocked a webview navigation to {shown}"));
                    false
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = dump(&args) {
        std::process::exit(code);
    }

    // Create the Nexulon-owned data folder and import any previous switcher's
    // data once.
    config::ensure_data_root();

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let icon = Icon::from_rgba(WINDOW_ICON.to_vec(), ICON_SIZE, ICON_SIZE)?;

    let window = WindowBuilder::new()
        .with_title("Nexulon Switcher")
        .with_inner_size(tao::dpi::LogicalSize::new(1120.0, 740.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(780.0, 520.0))
        .with_window_icon(Some(icon))
        .build(&event_loop)?;

    let proxy = event_loop.create_proxy();

    let mut web_context = wry::WebContext::new(Some(config::webview_data_dir()));
    let navigation = NavigationGuard::new();
    let webview = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_html(page())
        .with_background_color((6, 9, 18, 255))
        .with_devtools(cfg!(debug_assertions))
        .with_navigation_handler(move |url| navigation.allow(url))
        .with_ipc_handler(move |request| {
            let body = request.body().to_string();
            // Theme updates only need the window handle, so answer them on the
            // event loop instead of waking the switching engine.
            if let Ok(message) = serde_json::from_str::<Value>(&body) {
                if message.get("method").and_then(Value::as_str) == Some("window_theme") {
                    let params = message.get("params").cloned().unwrap_or(Value::Null);
                    let background = params
                        .get("bg")
                        .and_then(Value::as_str)
                        .unwrap_or("#060a14")
                        .to_string();
                    let text = params
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("#eef3ff")
                        .to_string();
                    let border = params
                        .get("border")
                        .and_then(Value::as_str)
                        .unwrap_or("#1a2338")
                        .to_string();
                    let _ = proxy.send_event(UserEvent::Theme(background, text, border));
                    let id = serde_json::to_string(&message.get("id").cloned().unwrap_or(Value::Null))
                        .unwrap_or_else(|_| "null".into());
                    let _ = proxy.send_event(UserEvent::RunScript(format!(
                        "window.__nexulonReply({id}, true, null);"
                    )));
                    return;
                }
            }
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                let script = {
                    let _guard = ENGINE_LOCK
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    answer(&body)
                };
                let _ = proxy.send_event(UserEvent::RunScript(script));
            });
        })
        .build(&window)?;

    apply_title_theme(&window, "#060a14", "#eef3ff", "#1a2338");

    // `NEXULON_EVAL` runs one JavaScript snippet once the page is up. It exists
    // so the interface can be driven by automated screenshot checks; it is
    // compiled out of release builds so the shipped exe cannot be scripted.
    #[cfg(debug_assertions)]
    if let Ok(script) = std::env::var("NEXULON_EVAL") {
        if !script.trim().is_empty() {
            let proxy = event_loop.create_proxy();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(2600));
                let _ = proxy.send_event(UserEvent::RunScript(script));
            });
        }
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(UserEvent::RunScript(script)) => {
                if !script.is_empty() {
                    let _ = webview.evaluate_script(&script);
                }
            }
            Event::UserEvent(UserEvent::Theme(background, text, border)) => {
                apply_title_theme(&window, &background, &text, &border);
            }
            _ => {}
        }
    });
}
