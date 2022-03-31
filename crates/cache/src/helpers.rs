use moon_logger::warn;
use std::env;

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
                    target: "moon:cache",
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

#[cfg(test)]
pub async fn run_with_env<T, F, Fut>(env: &str, callback: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    env::set_var("MOON_CACHE", env);

    let result = callback().await;

    env::remove_var("MOON_CACHE");

    result
}
