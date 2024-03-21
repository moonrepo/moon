use cached::proc_macro::cached;
use cargo_toml::Manifest as CargoToml;
use miette::IntoDiagnostic;
use moon_lang::config_cache_container;
use starbase_utils::glob;
use std::path::{Path, PathBuf};

pub use cargo_toml::*;

fn read_manifest(path: &Path) -> miette::Result<CargoToml> {
    CargoToml::from_path(path).into_diagnostic()
}

config_cache_container!(CargoTomlCache, CargoToml, "Cargo.toml", read_manifest);

pub trait CargoTomlExt {
    fn get_detailed_workspace_dependency(&self, name: &str) -> Option<DependencyDetail>;
    fn get_member_manifest_paths(&self, root_dir: &Path) -> miette::Result<Vec<PathBuf>>;
}

impl CargoTomlExt for CargoToml {
    #[allow(clippy::question_mark)]
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
            Dependency::Detailed(detail) => (**detail).to_owned(),
        })
    }

    fn get_member_manifest_paths(&self, root_dir: &Path) -> miette::Result<Vec<PathBuf>> {
        let mut paths = vec![];

        let Some(workspace) = &self.workspace else {
            return Ok(paths);
        };

        let mut patterns = workspace.members.clone();

        for exclude in &workspace.exclude {
            patterns.push(format!("!{exclude}"));
        }

        for manifest_dir in glob::walk(root_dir, &patterns)? {
            let manifest_path = manifest_dir.join("Cargo.toml");

            if manifest_path.exists() {
                paths.push(manifest_path);
            }
        }

        Ok(paths)
    }
}
