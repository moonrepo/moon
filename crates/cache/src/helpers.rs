use moon_logger::warn;
use std::env;
use std::time::SystemTime;

pub const LOG_TARGET: &str = "moon:cache";

static mut LOGGED_WARNING: bool = false;

pub fn get_cache_env_var() -> String {
    if let Ok(var) = env::var("MOON_CACHE") {
        if var == "off" || var == "read" || var == "write" {
            return var;
        }

        // We only want to show this once, not everytime the function is called
        unsafe {
            if !LOGGED_WARNING {
                LOGGED_WARNING = true;

                warn!(
                    target: LOG_TARGET,
                    "Unknown MOON_CACHE environment variable value \"{}\", falling back to write mode",
                    var
                );
            }
        }
    }

    String::from("write")
}

pub fn is_readable() -> bool {
    get_cache_env_var() != "off"
}

pub fn is_writable() -> bool {
    get_cache_env_var() == "write"
}

pub fn to_millis(time: SystemTime) -> u128 {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_millis(),
        Err(_) => 0,
    }
}

#[cfg(test)]
pub async fn run_with_env<T, F, Fut>(env: &str, callback: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    if env.is_empty() {
        env::remove_var("MOON_CACHE");
    } else {
        env::set_var("MOON_CACHE", env);
    }

    let result = callback().await;

    env::remove_var("MOON_CACHE");

    result
}
