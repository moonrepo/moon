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
    pub version: Option<String>,
}

impl NodeLanguage {
    pub fn new(proto: &Proto) -> Self {
        NodeLanguage {
            base_dir: proto.tools_dir.join("node"),
            bin_path: None,
            log_target: "proto:tool:node".into(),
            shim_path: None,
            temp_dir: proto.temp_dir.join("node"),
            version: None,
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
