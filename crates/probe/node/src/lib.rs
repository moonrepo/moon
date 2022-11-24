// mod depman;
pub mod download;
mod install;
mod platform;
mod resolve;
mod verify;

// pub use depman::*;

use probe_core::{Describable, Probe, Tool};
use std::path::PathBuf;

pub struct NodeLanguage {
    pub install_dir: PathBuf,
    pub log_target: String,
    pub temp_dir: PathBuf,
    pub version: String,
}

impl NodeLanguage {
    pub fn new(probe: &Probe, version: Option<&str>) -> Self {
        NodeLanguage {
            install_dir: probe.tools_dir.join("node"),
            log_target: "probe:tool:node".into(),
            temp_dir: probe.temp_dir.join("node"),
            version: version.unwrap_or("latest").into(),
        }
    }
}

impl Describable<'_> for NodeLanguage {
    fn get_log_target(&self) -> &str {
        &self.log_target
    }

    fn get_name(&self) -> String {
        "Node.js".into()
    }
}

impl Tool<'_> for NodeLanguage {}
