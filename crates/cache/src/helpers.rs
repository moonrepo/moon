use std::env;

pub fn is_readable() -> bool {
    match env::var("MOON_CACHE") {
        Ok(var) => var == "read" || var == "write",
        Err(_) => true,
    }
}

pub fn is_writable() -> bool {
    match env::var("MOON_CACHE") {
        Ok(var) => var == "write",
        Err(_) => true,
    }
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
