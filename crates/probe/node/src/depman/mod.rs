// mod download;
// mod install;
// mod platform;
mod resolve;
// mod verify;

use probe_core::{Probe, Tool};
use resolve::NDMVersionDist;
use std::{marker::PhantomData, path::PathBuf};

pub enum NodeDependencyManagerType {
    Npm,
    Pnpm,
    Yarn,
}

impl NodeDependencyManagerType {
    pub fn get_package_name(&self) -> String {
        match self {
            NodeDependencyManagerType::Npm => "npm".into(),
            NodeDependencyManagerType::Pnpm => "pnpm".into(),
            NodeDependencyManagerType::Yarn => "yarn".into(),
        }
    }
}

pub struct NodeDependencyManager<'tool> {
    pub dist: Option<NDMVersionDist>,
    pub install_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub type_of: NodeDependencyManagerType,
    pub version: String,

    #[allow(dead_code)]
    data: PhantomData<&'tool ()>,
}

impl<'tool> NodeDependencyManager<'tool> {
    pub fn new(probe: &Probe, type_of: NodeDependencyManagerType, version: &str) -> Self {
        let package_name = type_of.get_package_name();

        NodeDependencyManager {
            dist: None,
            install_dir: probe.tools_dir.join(&package_name),
            temp_dir: probe.temp_dir.join(&package_name),
            type_of,
            version: version.to_owned(),
            data: PhantomData,
        }
    }
}

impl<'tool> Tool<'tool> for NodeDependencyManager<'tool> {}
