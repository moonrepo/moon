mod engine;
mod hasher;
mod items;
mod runfiles;

pub use engine::CacheEngine;
pub use hasher::Hasher;
pub use items::*;
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
