use crate::CARGO;
use cached::proc_macro::cached;
use cargo_toml::Manifest as CargoToml;
use moon_error::MoonError;
use moon_lang::config_cache_container;
use std::path::{Path, PathBuf};

pub use cargo_toml::*;

fn read_manifest(path: &Path) -> Result<CargoToml, MoonError> {
    CargoToml::from_path(path).map_err(|e| MoonError::Generic(e.to_string()))
}

config_cache_container!(CargoTomlCache, CargoToml, CARGO.manifest, read_manifest);
