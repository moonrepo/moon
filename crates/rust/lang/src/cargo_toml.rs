use crate::CARGO;
use cached::proc_macro::cached;
use cargo_toml::Manifest as CargoToml;
use moon_error::MoonError;
use moon_lang::config_cache_container;
use starbase_utils::toml::read_file as read_toml;
use std::path::{Path, PathBuf};

config_cache_container!(CargoTomlCache, CargoToml, CARGO.manifest, read_toml);
