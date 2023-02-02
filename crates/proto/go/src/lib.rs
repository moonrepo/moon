mod detect;
pub mod download;
mod execute;
mod install;
mod platform;
mod resolve;
mod shim;
mod verify;

use proto_core::{Describable, Proto, Tool};
use std::path::PathBuf;

#[derive(Debug)]
pub struct GoLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub log_target: String,
    pub shim_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,
}

impl GoLanguage {
    pub fn new(proto: &Proto) -> Self {
        GoLanguage {
            base_dir: proto.tools_dir.join("go"),
            bin_path: None,
            log_target: "proto:tool:go".into(),
            shim_path: None,
            temp_dir: proto.temp_dir.join("go"),
            version: None,
        }
    }
}

impl Describable<'_> for GoLanguage {
    fn get_bin_name(&self) -> &str {
        "go"
    }

    fn get_log_target(&self) -> &str {
        &self.log_target
    }

    fn get_name(&self) -> String {
        "Go".into()
    }
}

impl Tool<'_> for GoLanguage {}
