pub mod fs;
pub mod path;
pub mod process;
pub mod regex;
pub mod test;
pub mod time;

use cached::proc_macro::cached;
use std::env;
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::Duration;

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

pub fn is_ci() -> bool {
    env::var("CI").is_ok()
}

#[cached(time = 60)]
pub fn is_offline() -> bool {
    // Cloudflare's DNS: https://1.1.1.1/dns/
    let address = SocketAddr::from(([1, 1, 1, 1], 53));
    let mut offline = true;

    if let Ok(stream) = TcpStream::connect_timeout(&address, Duration::new(3, 0)) {
        stream.shutdown(Shutdown::Both).unwrap();
        offline = false;
    }

    offline
}
