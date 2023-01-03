pub mod depman;
mod detect;
pub mod download;
mod execute;
mod install;
mod platform;
mod resolve;
mod shim;
mod verify;

pub use depman::*;
use proto_core::{Describable, Proto, Tool};
use std::path::PathBuf;

#[derive(Debug)]
pub struct NodeLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub log_target: String,
    pub shim_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: String,
}

impl NodeLanguage {
    pub fn new(proto: &Proto, version: Option<&str>) -> Self {
        NodeLanguage {
            bin_path: None,
            base_dir: proto.tools_dir.join("node"),
            log_target: "proto:tool:node".into(),
            shim_path: None,
            temp_dir: proto.temp_dir.join("node"),
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
