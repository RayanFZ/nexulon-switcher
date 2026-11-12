//! Minimal HTTPS client built on WinHTTP.
//!
//! The app only needs a handful of JSON and image requests, so pulling in a
//! full HTTP stack (and its TLS library) would add megabytes for no benefit.
//! WinHTTP is part of Windows, uses the system proxy and certificate store,
//! and costs nothing in binary size.

use std::ffi::c_void;

type Handle = *mut c_void;

#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpOpen(
        agent: *const u16,
        access_type: u32,
        proxy: *const u16,
        bypass: *const u16,
        flags: u32,
    ) -> Handle;
    fn WinHttpConnect(session: Handle, host: *const u16, port: u16, reserved: u32) -> Handle;
    fn WinHttpOpenRequest(
        connect: Handle,
        verb: *const u16,
        path: *const u16,
        version: *const u16,
        referrer: *const u16,
        accept_types: *const *const u16,
        flags: u32,
    ) -> Handle;
    fn WinHttpSetTimeouts(
        handle: Handle,
        resolve: i32,
        connect: i32,
        send: i32,
        receive: i32,
    ) -> i32;
    fn WinHttpSetOption(handle: Handle, option: u32, buffer: *mut c_void, len: u32) -> i32;
    fn WinHttpSendRequest(
        request: Handle,
        headers: *const u16,
        headers_len: u32,
        optional: *mut c_void,
        optional_len: u32,
        total_len: u32,
        context: usize,
    ) -> i32;
    fn WinHttpReceiveResponse(request: Handle, reserved: *mut c_void) -> i32;
    fn WinHttpQueryHeaders(
        request: Handle,
        info_level: u32,
        name: *const u16,
        buffer: *mut c_void,
        buffer_len: *mut u32,
        index: *mut u32,
    ) -> i32;
    fn WinHttpReadData(request: Handle, buffer: *mut c_void, to_read: u32, read: *mut u32) -> i32;
    fn WinHttpCloseHandle(handle: Handle) -> i32;
}

const ACCESS_TYPE_DEFAULT_PROXY: u32 = 0;
const ACCESS_TYPE_NO_PROXY: u32 = 1;
const FLAG_SECURE: u32 = 0x0080_0000;
const QUERY_STATUS_CODE: u32 = 19;
const QUERY_FLAG_NUMBER: u32 = 0x2000_0000;
const MAX_BODY: usize = 24 * 1024 * 1024;

// The Riot Client's local API uses a self-signed certificate. Only requests
// made through `request_local` relax certificate checks, and only to loopback.
const OPTION_SECURITY_FLAGS: u32 = 31;
const IGNORE_UNKNOWN_CA: u32 = 0x0000_0100;
const IGNORE_CERT_CN_INVALID: u32 = 0x0000_1000;
const IGNORE_CERT_DATE_INVALID: u32 = 0x0000_2000;
const IGNORE_CERT_WRONG_USAGE: u32 = 0x0000_0200;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

struct HandleGuard(Handle);

impl Drop for HandleGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }
}

struct Url {
    host: String,
    port: u16,
    path: String,
    secure: bool,
}

fn parse_url(url: &str) -> Result<Url, String> {
    let (rest, secure, default_port) = if let Some(rest) = url.strip_prefix("https://") {
        (rest, true, 443)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (rest, false, 80)
    } else {
        return Err("Only http:// and https:// URLs are supported.".into());
    };
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return Err("The URL has no host.".into());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => (
            host,
            port.parse::<u16>()
                .map_err(|_| "The URL port is invalid.".to_string())?,
        ),
        _ => (authority, default_port),
    };
    Ok(Url {
        host: host.to_string(),
        port,
        path: path.to_string(),
        secure,
    })
}

pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Perform one HTTP(S) request. Redirects are followed by WinHTTP.
pub fn request(
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> Result<Response, String> {
    send(method, url, headers, body, false)
}

/// A request to a loopback service with a self-signed certificate, such as the
/// Riot Client's local API. Proxies and certificate checks are bypassed only
/// here, and only for a loopback host, so ordinary internet requests keep full
/// validation.
pub fn request_local(
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> Result<Response, String> {
    send(method, url, headers, body, true)
}

/// True for `localhost`, `127.0.0.0/8` and `::1` — the only hosts allowed to
/// skip certificate validation.
fn is_loopback(host: &str) -> bool {
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<std::net::Ipv4Addr>()
        .map(|address| address.is_loopback())
        .or_else(|_| host.parse::<std::net::Ipv6Addr>().map(|address| address.is_loopback()))
        .unwrap_or(false)
}

fn send(
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
    local: bool,
) -> Result<Response, String> {
    let parsed = parse_url(url)?;
    if local && !is_loopback(&parsed.host) {
        return Err(format!(
            "Refusing to relax certificate checks for {}, which is not a loopback host.",
            parsed.host
        ));
    }
    let agent = wide("NexulonSwitcher/1.0");
    let session = unsafe {
        WinHttpOpen(
            agent.as_ptr(),
            if local {
                ACCESS_TYPE_NO_PROXY
            } else {
                ACCESS_TYPE_DEFAULT_PROXY
            },
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    };
    if session.is_null() {
        return Err(format!("WinHTTP could not start: {}", std::io::Error::last_os_error()));
    }
    let session = HandleGuard(session);
    unsafe {
        WinHttpSetTimeouts(session.0, 10_000, 10_000, 20_000, 25_000);
    }

    let host = wide(&parsed.host);
    let connect = unsafe { WinHttpConnect(session.0, host.as_ptr(), parsed.port, 0) };
    if connect.is_null() {
        return Err(format!("Could not reach {}: {}", parsed.host, std::io::Error::last_os_error()));
    }
    let connect = HandleGuard(connect);

    let verb = wide(method);
    let path = wide(&parsed.path);
    let request = unsafe {
        WinHttpOpenRequest(
            connect.0,
            verb.as_ptr(),
            path.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            if parsed.secure { FLAG_SECURE } else { 0 },
        )
    };
    if request.is_null() {
        return Err(format!("Could not open the request: {}", std::io::Error::last_os_error()));
    }
    let request = HandleGuard(request);

    if local && parsed.secure {
        let mut flags = IGNORE_UNKNOWN_CA
            | IGNORE_CERT_CN_INVALID
            | IGNORE_CERT_DATE_INVALID
            | IGNORE_CERT_WRONG_USAGE;
        unsafe {
            WinHttpSetOption(
                request.0,
                OPTION_SECURITY_FLAGS,
                &mut flags as *mut u32 as *mut c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }

    let mut header_text = String::new();
    for (name, value) in headers {
        header_text.push_str(name);
        header_text.push_str(": ");
        header_text.push_str(value);
        header_text.push_str("\r\n");
    }
    let header_wide = wide(&header_text);
    let (body_ptr, body_len) = match body {
        Some(bytes) => (bytes.as_ptr() as *mut c_void, bytes.len() as u32),
        None => (std::ptr::null_mut(), 0),
    };

    let sent = unsafe {
        WinHttpSendRequest(
            request.0,
            header_wide.as_ptr(),
            (header_wide.len() - 1) as u32,
            body_ptr,
            body_len,
            body_len,
            0,
        )
    };
    if sent == 0 {
        return Err(format!(
            "The request to {} failed: {}",
            parsed.host,
            std::io::Error::last_os_error()
        ));
    }
    if unsafe { WinHttpReceiveResponse(request.0, std::ptr::null_mut()) } == 0 {
        return Err(format!(
            "No response from {}: {}",
            parsed.host,
            std::io::Error::last_os_error()
        ));
    }

    let mut status: u32 = 0;
    let mut status_len = std::mem::size_of::<u32>() as u32;
    let ok = unsafe {
        WinHttpQueryHeaders(
            request.0,
            QUERY_STATUS_CODE | QUERY_FLAG_NUMBER,
            std::ptr::null(),
            &mut status as *mut u32 as *mut c_void,
            &mut status_len,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err("The server sent no status code.".into());
    }

    let mut body_bytes = Vec::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let mut read: u32 = 0;
        let ok = unsafe {
            WinHttpReadData(
                request.0,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                &mut read,
            )
        };
        if ok == 0 {
            return Err(format!("The download stopped early: {}", std::io::Error::last_os_error()));
        }
        if read == 0 {
            break;
        }
        body_bytes.extend_from_slice(&buffer[..read as usize]);
        if body_bytes.len() > MAX_BODY {
            return Err("That file is too large to download.".into());
        }
    }

    Ok(Response {
        status: status as u16,
        body: body_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::is_loopback;

    #[test]
    fn loopback_hosts_are_accepted() {
        for host in ["127.0.0.1", "127.4.5.6", "localhost", "LOCALHOST", "::1", "[::1]"] {
            assert!(is_loopback(host), "{host} should count as loopback");
        }
    }

    #[test]
    fn public_hosts_are_rejected() {
        for host in [
            "riotgames.com",
            "localhost.example.com",
            "10.0.0.1",
            "192.168.1.10",
            "0.0.0.0",
            "",
        ] {
            assert!(!is_loopback(host), "{host} must not count as loopback");
        }
    }
}
