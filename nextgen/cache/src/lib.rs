mod cache_engine;

pub use cache_engine::*;
pub use moon_cache_item::*;

use moon_common::consts::CONFIG_DIRNAME;
use starbase_utils::dirs;
use std::path::PathBuf;

#[inline]
pub fn get_moon_home_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Invalid home directory.")
        .join(CONFIG_DIRNAME)
}
