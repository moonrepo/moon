// mod depman;
pub mod download;
mod install;
mod platform;
mod resolve;
mod verify;

// pub use depman::*;

use probe_core::{Probe, Tool};
use std::path::PathBuf;

pub struct NodeLanguage {
    pub install_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub version: String,
}

impl NodeLanguage {
    pub fn new(probe: &Probe, version: Option<&str>) -> Self {
        NodeLanguage {
            install_dir: probe.tools_dir.join("node"),
            temp_dir: probe.temp_dir.join("node"),
            version: version.unwrap_or("latest").into(),
        }
    }
}

impl Tool<'_> for NodeLanguage {}
