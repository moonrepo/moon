use moon_common::{Id, color};
use moon_config::{BunConfig, DependencyScope, NodeConfig, NodeVersionFormat};
use moon_node_lang::PackageJsonCache;
use moon_project::Project;
use moon_utils::{path, semver};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::debug;

pub struct JavaScriptSyncer<'app> {
    bun: bool,
    project: &'app Project,

    // Settings
    dependency_version_format: NodeVersionFormat,
    sync_project_workspace_dependencies: bool,
}

impl<'app> JavaScriptSyncer<'app> {
    pub fn for_bun(project: &'app Project, bun_config: &'app BunConfig) -> Self {
        Self {
            bun: true,
            dependency_version_format: bun_config.dependency_version_format,
            sync_project_workspace_dependencies: bun_config.sync_project_workspace_dependencies,
            project,
        }
    }

    pub fn for_node(project: &'app Project, node_config: &'app NodeConfig) -> Self {
        Self {
            bun: false,
            dependency_version_format: node_config.dependency_version_format,
            sync_project_workspace_dependencies: node_config.sync_project_workspace_dependencies,
            project,
        }
    }

    pub fn sync(&self, dependencies: &FxHashMap<Id, Arc<Project>>) -> miette::Result<bool> {
        let mut mutated = false;

        if !self.sync_project_workspace_dependencies || self.project.is_root_level() {
            return Ok(mutated);
        }

        // Sync each dependency to `package.json`
        let mut package_prod_deps: BTreeMap<String, String> = BTreeMap::new();
        let mut package_peer_deps: BTreeMap<String, String> = BTreeMap::new();
        let mut package_dev_deps: BTreeMap<String, String> = BTreeMap::new();
        let version_prefix = self.dependency_version_format.get_prefix();

        for dep_config in &self.project.dependencies {
            let Some(dep_project) = dependencies.get(&dep_config.id) else {
                continue;
            };

            if dep_project.is_root_level() || dep_config.is_root_scope() {
                continue;
            }

            // Update dependencies within this project's `package.json`.
            // Only add if the dependent project has a `package.json`,
            // and this `package.json` has not already declared the dep.
            if let Some(dep_package_json) = PackageJsonCache::read(&dep_project.root)? {
                if let Some(dep_package_name) = &dep_package_json.data.name {
                    let dep_package_version = dep_package_json.data.version.unwrap_or_default();
                    let dep_version = match &self.dependency_version_format {
                        NodeVersionFormat::File | NodeVersionFormat::Link => {
                            format!(
                                "{}{}",
                                version_prefix,
                                // https://bun.sh/docs/cli/link
                                if self.bun
                                    && self.dependency_version_format == NodeVersionFormat::Link
                                {
                                    dep_package_name.to_owned()
                                } else {
                                    path::to_relative_virtual_string(
                                        &dep_project.root,
                                        &self.project.root,
                                    )?
                                }
                            )
                        }
                        NodeVersionFormat::Version
                        | NodeVersionFormat::VersionCaret
                        | NodeVersionFormat::VersionTilde => {
                            format!("{version_prefix}{dep_package_version}")
                        }
                        _ => version_prefix.clone(),
                    };

                    match dep_config.scope {
                        DependencyScope::Build | DependencyScope::Root => {
                            // Not supported
                        }
                        DependencyScope::Production => {
                            package_prod_deps.insert(dep_package_name.to_owned(), dep_version);
                        }
                        DependencyScope::Development => {
                            package_dev_deps.insert(dep_package_name.to_owned(), dep_version);
                        }
                        DependencyScope::Peer => {
                            // Peers are unique, so lets handle this manually here for now.
                            // Perhaps we can wrap this in a new setting in the future.
                            package_peer_deps.insert(
                                dep_package_name.to_owned(),
                                format!(
                                    "^{}.0.0",
                                    semver::extract_major_version(dep_package_version)
                                ),
                            );
                        }
                    }

                    debug!(
                        scope = ?dep_config.scope,
                        "Syncing {} as a dependency to {}'s package.json",
                        color::id(&dep_project.id),
                        color::id(&self.project.id),
                    );
                }
            }
        }

        // Sync to the project's `package.json`
        if !package_prod_deps.is_empty()
            || !package_dev_deps.is_empty()
            || !package_peer_deps.is_empty()
        {
            PackageJsonCache::sync(&self.project.root, |package_json| {
                let mut mutated_package = false;

                for (name, version) in package_prod_deps {
                    if package_json.add_dependency(&name, &version, true) {
                        mutated_package = true;
                    }
                }

                for (name, version) in package_dev_deps {
                    if package_json.add_dev_dependency(&name, &version, true) {
                        mutated_package = true;
                    }
                }

                for (name, version) in package_peer_deps {
                    if package_json.add_peer_dependency(&name, &version, true) {
                        mutated_package = true;
                    }
                }

                if mutated_package {
                    mutated = true;
                }

                Ok(mutated_package)
            })?;
        }

        Ok(mutated)
    }
}
