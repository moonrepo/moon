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

pub trait CargoTomlExt {
    fn get_detailed_workspace_dependency(&self, name: &str) -> Option<DependencyDetail>;
}

impl CargoTomlExt for CargoToml {
    fn get_detailed_workspace_dependency(&self, name: &str) -> Option<DependencyDetail> {
        let Some(workspace) = &self.workspace else {
            return None;
        };

        workspace.dependencies.get(name).map(|dep| match dep {
            Dependency::Simple(version) => DependencyDetail {
                version: Some(version.to_owned()),
                ..DependencyDetail::default()
            },
            Dependency::Inherited(data) => DependencyDetail {
                features: data.features.clone(),
                optional: data.optional,
                ..DependencyDetail::default()
            },
            Dependency::Detailed(detail) => detail.to_owned(),
        })
    }
}
