//! Platform artwork compiled straight into the executable.
//!
//! Keeping the icons inside the binary means the portable build is a single
//! file and the installed build never has to look for a sidecar folder.

#[derive(rust_embed::RustEmbed)]
#[folder = "assets/platforms"]
struct PlatformIcons;

fn lookup(name: &str) -> Option<String> {
    let candidates = [
        format!("{name}.svg"),
        format!("{}.svg", name.replace(':', "")),
        format!("{}.svg", name.replace(':', "").replace("  ", " ")),
    ];

    fn clean(data: &[u8]) -> String {
        // Some editors save SVG files with a UTF-8 BOM. The front end only
        // treats a value as inline SVG when it starts with `<svg`, so strip it.
        String::from_utf8_lossy(data)
            .trim_start_matches('\u{feff}')
            .trim_start()
            .to_string()
    }

    for candidate in &candidates {
        if let Some(file) = PlatformIcons::get(candidate) {
            return Some(clean(&file.data));
        }
    }

    // Case-insensitive fallback for hand-edited platform names.
    for candidate in &candidates {
        if let Some(path) = PlatformIcons::iter().find(|path| path.eq_ignore_ascii_case(candidate)) {
            if let Some(file) = PlatformIcons::get(&path) {
                return Some(clean(&file.data));
            }
        }
    }
    None
}

/// Raw SVG source for a platform, if one was shipped.
pub fn platform_svg(name: &str) -> Option<String> {
    lookup(name)
}

/// The Nexulon mark as a data URI, so the header needs no extra file on disk.
pub fn logo_data_uri() -> String {
    use base64::Engine;
    let bytes = include_bytes!("../assets/nexulon.png");
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}
