use crate::CARGO;
use cached::proc_macro::cached;
use cargo_lock::Lockfile as CargoLock;
use moon_error::MoonError;
use moon_lang::config_cache_container;
use std::path::{Path, PathBuf};

fn read_lockfile(path: &Path) -> Result<CargoLock, MoonError> {
    CargoLock::load(path).map_err(|e| MoonError::Generic(e.to_string()))
}

config_cache_container!(CargoLockCache, CargoLock, CARGO.lockfile, read_lockfile);
