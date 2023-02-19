use crate::ProtoError;
use cached::proc_macro::cached;
use dirs::home_dir;
use std::{env, path::PathBuf};

pub fn get_root() -> Result<PathBuf, ProtoError> {
    if let Ok(root) = env::var("PROTO_ROOT") {
        return Ok(root.into());
    }

    if let Some(dir) = home_dir() {
        return Ok(dir.join(".proto"));
    }

    Err(ProtoError::MissingHomeDir)
}

pub fn get_shims_dir() -> Result<PathBuf, ProtoError> {
    Ok(get_root()?.join("shims"))
}

pub fn get_temp_dir() -> Result<PathBuf, ProtoError> {
    Ok(get_root()?.join("temp"))
}

pub fn get_tools_dir() -> Result<PathBuf, ProtoError> {
    Ok(get_root()?.join("tools"))
}

// Aliases are words that map to version. For example, "latest" -> "1.2.3".
pub fn is_version_alias(value: &str) -> bool {
    value
        .chars()
        .all(|c| char::is_ascii_alphabetic(&c) || c == '-')
}

pub fn add_v_prefix(value: &str) -> String {
    if value.starts_with('v') || value.starts_with('V') {
        return value.to_lowercase();
    }

    format!("v{value}")
}

pub fn remove_v_prefix(value: &str) -> String {
    if value.starts_with('v') || value.starts_with('V') {
        return value[1..].to_owned();
    }

    value.to_owned()
}

#[cached(time = 300)]
pub fn is_offline() -> bool {
    use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let mut addresses = vec![];

    if let Ok(addrs) = "google.com:80".to_socket_addrs() {
        addresses.extend(addrs);
    }

    addresses.extend([
        // Cloudflare DNS: https://1.1.1.1/dns/
        SocketAddr::from(([1, 1, 1, 1], 53)),
        SocketAddr::from(([1, 0, 0, 1], 53)),
        // Google DNS: https://developers.google.com/speed/public-dns
        SocketAddr::from(([8, 8, 8, 8], 53)),
        SocketAddr::from(([8, 8, 4, 4], 53)),
    ]);

    for address in addresses {
        if TcpStream::connect_timeout(&address, Duration::new(3, 0)).is_ok() {
            return false;
        }
    }

    true
}
