mod depman;
mod download;
mod install;
mod platform;
mod resolve;
mod verify;

pub use depman::*;

use probe_core::{Probe, Tool};
use std::{marker::PhantomData, path::PathBuf};

pub struct NodeLanguage<'tool> {
    pub install_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub version: String,

    #[allow(dead_code)]
    data: PhantomData<&'tool ()>,
}

impl<'tool> NodeLanguage<'tool> {
    pub fn new(probe: &Probe, version: &str) -> Self {
        NodeLanguage {
            install_dir: probe.tools_dir.join("node"),
            temp_dir: probe.temp_dir.join("node"),
            version: version.to_owned(),
            data: PhantomData,
        }
    }
}

impl<'tool> Tool<'tool> for NodeLanguage<'tool> {}
