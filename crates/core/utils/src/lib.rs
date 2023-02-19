pub mod fs;
pub mod glob;
pub mod json;
pub mod path;
pub mod process;
pub mod regex;
pub mod semver;
pub mod shell;
pub mod time;
pub mod yaml;

pub use async_trait::async_trait;
pub use lazy_static::lazy_static;

use cached::proc_macro::cached;
use moon_constants as constants;
use std::env;
use std::path::PathBuf;

#[macro_export]
macro_rules! string_vec {
    () => {{
        Vec::<String>::new()
    }};
    ($($item:expr),+ $(,)?) => {{
        vec![
            $( String::from($item), )*
        ]
    }};
}

#[cached]
pub fn get_workspace_root() -> PathBuf {
    if let Ok(root) = env::var("MOON_WORKSPACE_ROOT") {
        let root: PathBuf = root.parse().expect("Failed to parse MOON_WORKSPACE_ROOT.");

        return root;
    }

    match fs::find_upwards(
        constants::CONFIG_DIRNAME,
        env::current_dir().expect("Invalid working directory."),
    ) {
        Some(dir) => dir.parent().unwrap().to_path_buf(),
        None => panic!("Unable to get workspace root. Is moon running?"),
    }
}

#[inline]
pub fn get_cache_dir() -> PathBuf {
    get_workspace_root()
        .join(constants::CONFIG_DIRNAME)
        .join("cache")
}

#[inline]
pub fn is_ci() -> bool {
    match env::var("CI") {
        Ok(var) => var == "true",
        Err(_) => false,
    }
}

#[inline]
pub fn is_docker_container() -> bool {
    PathBuf::from("/.dockerenv").exists()
}

#[cached(time = 300)]
pub fn is_offline() -> bool {
    if let Ok(value) = env::var("MOON_OFFLINE") {
        match value.as_ref() {
            "1" | "true" => return true,
            "0" | "false" => return false,
            _ => {}
        };
    }

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

#[inline]
pub fn is_test_env() -> bool {
    env::var("MOON_TEST").is_ok()
}
