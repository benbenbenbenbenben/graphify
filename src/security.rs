use regex::Regex;
use std::collections::HashSet;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use thiserror::Error;
use url::Url;

pub const MAX_FETCH_BYTES: usize = 52_428_800; // 50 MB
pub const MAX_TEXT_BYTES: usize = 10_485_760;  // 10 MB

lazy_static::lazy_static! {
    static ref ALLOWED_SCHEMES: HashSet<&'static str> = vec!["http", "https"].into_iter().collect();
    static ref BLOCKED_HOSTS: HashSet<&'static str> = vec!["metadata.google.internal", "metadata.google.com"].into_iter().collect();
    static ref CONTROL_CHAR_RE: Regex = Regex::new(r"[\x00-\x1f\x7f]").unwrap();
}

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Blocked URL scheme '{0}' - only http and https are allowed.")]
    InvalidScheme(String),
    #[error("Blocked cloud metadata endpoint '{0}'.")]
    BlockedHost(String),
    #[error("Blocked private/internal IP {0}.")]
    BlockedIp(String),
    #[error("Invalid URL: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Response exceeds size limit")]
    SizeLimitExceeded,
    #[error("Path escapes allowed directory")]
    PathEscapesBase,
    #[error("Base directory does not exist: {0}")]
    BaseDoesNotExist(String),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

fn is_private_or_internal(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            ipv4.is_private()
                || ipv4.is_loopback()
                || ipv4.is_link_local()
                || ipv4.is_unspecified()
                || ipv4.is_multicast()
                || ipv4.is_broadcast()
                // Carrier-grade NAT (100.64.0.0/10)
                || (octets[0] == 100 && (octets[1] & 0b1100_0000 == 0b0100_0000))
                // Reserved / future use (240.0.0.0/4 and 0.0.0.0/8)
                || octets[0] >= 240
                || octets[0] == 0
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()
                || ipv6.is_unspecified()
                || ipv6.is_multicast()
                // Link-local (fe80::/10)
                || (ipv6.segments()[0] & 0xffc0) == 0xfe80
                // Unique local (fc00::/7)
                || (ipv6.segments()[0] & 0xfe00) == 0xfc00
        }
    }
}

pub fn validate_url(url_str: &str) -> Result<String, SecurityError> {
    let parsed = Url::parse(url_str)?;
    let scheme = parsed.scheme().to_lowercase();

    if !ALLOWED_SCHEMES.contains(scheme.as_str()) {
        return Err(SecurityError::InvalidScheme(scheme));
    }

    if let Some(host) = parsed.host_str() {
        if BLOCKED_HOSTS.contains(host.to_lowercase().as_str()) {
            return Err(SecurityError::BlockedHost(host.to_string()));
        }

        if let Ok(addrs) = format!("{}:80", host).to_socket_addrs() {
            for addr in addrs {
                if is_private_or_internal(&addr.ip()) {
                    return Err(SecurityError::BlockedIp(addr.ip().to_string()));
                }
            }
        }
    }

    Ok(url_str.to_string())
}

pub fn safe_fetch(url: &str, max_bytes: usize) -> Result<Vec<u8>, SecurityError> {
    validate_url(url)?;

    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 graphify/1.0")
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if validate_url(attempt.url().as_str()).is_err() {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()?;

    let mut response = client.get(url).send()?;
    let status = response.status();
    if !status.is_success() {
        return Err(SecurityError::NetworkError(response.error_for_status().unwrap_err()));
    }

    let mut buf = Vec::new();
    use std::io::Read;
    let mut chunk = [0; 65536];

    loop {
        let n = response.read(&mut chunk)?;
        if n == 0 { break; }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > max_bytes {
            return Err(SecurityError::SizeLimitExceeded);
        }
    }

    Ok(buf)
}

pub fn safe_fetch_text(url: &str, max_bytes: usize) -> Result<String, SecurityError> {
    let bytes = safe_fetch(url, max_bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn validate_graph_path(path: &Path, base: Option<&Path>) -> Result<PathBuf, SecurityError> {
    let base_path = base.unwrap_or(Path::new("graphify-out")).to_path_buf();

    let abs_base = std::fs::canonicalize(&base_path).map_err(|_| {
        SecurityError::BaseDoesNotExist(base_path.to_string_lossy().into_owned())
    })?;

    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path)
    };

    // Normalize by resolving ".." components (without canonicalization) so that
    // directory-traversal attempts like "base/../secret" are caught before any I/O.
    let mut resolved = PathBuf::new();
    for comp in abs_path.components() {
        if comp.as_os_str() == ".." {
            resolved.pop();
        } else if comp.as_os_str() != "." {
            resolved.push(comp);
        }
    }

    // Reject paths that escape the base directory even before the file exists.
    if !resolved.starts_with(&abs_base) {
        return Err(SecurityError::PathEscapesBase);
    }

    if !resolved.exists() {
        return Err(SecurityError::FileNotFound(resolved.to_string_lossy().into_owned()));
    }

    // Canonicalize *after* the existence check to resolve symlinks and catch
    // symlink-based escape attempts that bypass the lexical check above.
    let canonical = std::fs::canonicalize(&resolved)?;
    if !canonical.starts_with(&abs_base) {
        return Err(SecurityError::PathEscapesBase);
    }

    Ok(canonical)
}

pub fn sanitize_label(text: &str) -> String {
    let cleaned = CONTROL_CHAR_RE.replace_all(text, "").to_string();
    let limit = 256.min(cleaned.chars().count());
    cleaned.chars().take(limit).collect()
}
