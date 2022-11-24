mod download;
mod install;
mod resolve;
mod verify;

use probe_core::{Describable, Probe, Tool};
use resolve::NDMVersionDist;
use std::path::PathBuf;

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

pub struct NodeDependencyManager {
    pub dist: Option<NDMVersionDist>,
    pub install_dir: PathBuf,
    pub log_target: String,
    pub temp_dir: PathBuf,
    pub type_of: NodeDependencyManagerType,
    pub version: String,
}

impl NodeDependencyManager {
    pub fn new(probe: &Probe, type_of: NodeDependencyManagerType, version: Option<&str>) -> Self {
        let package_name = type_of.get_package_name();

        NodeDependencyManager {
            dist: None,
            install_dir: probe.tools_dir.join(&package_name),
            log_target: format!("probe:tool:{}", &package_name),
            temp_dir: probe.temp_dir.join(&package_name),
            type_of,
            version: version.unwrap_or("latest").into(),
        }
    }

    pub fn get_dist(&self) -> &NDMVersionDist {
        self.dist
            .as_ref()
            .expect("Distribution info not defined for node dependency manager!")
    }
}

impl Describable<'_> for NodeDependencyManager {
    fn get_log_target(&self) -> &str {
        &self.log_target
    }

    fn get_name(&self) -> String {
        self.type_of.get_package_name()
    }
}

impl Tool<'_> for NodeDependencyManager {}
