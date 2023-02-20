use moon_config::{DependencyScope, NodeConfig, NodeVersionFormat, TypeScriptConfig};
use moon_logger::{color, debug};
use moon_node_lang::{PackageJson, NPM};
use moon_project::{Project, ProjectError};
use moon_typescript_lang::tsconfig::CompilerOptionsPaths;
use moon_utils::{path, semver};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::path::Path;

const LOG_TARGET: &str = "moon:node-platform:sync-project";

pub async fn sync_project(
    project: &Project,
    dependencies: &FxHashMap<String, &Project>,
    workspace_root: &Path,
    node_config: &NodeConfig,
    typescript_config: &Option<TypeScriptConfig>,
) -> Result<bool, ProjectError> {
    let mut mutated_project_files = false;
    let is_project_typescript_enabled = project.config.toolchain.is_typescript_enabled();

    // Sync each dependency to `tsconfig.json` and `package.json`
    let mut package_prod_deps: BTreeMap<String, String> = BTreeMap::new();
    let mut package_peer_deps: BTreeMap<String, String> = BTreeMap::new();
    let mut package_dev_deps: BTreeMap<String, String> = BTreeMap::new();
    let mut tsconfig_project_refs: FxHashSet<String> = FxHashSet::default();
    let mut tsconfig_paths: CompilerOptionsPaths = BTreeMap::new();

    for (dep_id, dep_cfg) in &project.dependencies {
        let Some(dep_project) = dependencies.get(dep_id) else {
            continue;
        };

        let dep_relative_path =
            path::relative_from(&dep_project.root, &project.root).unwrap_or_default();
        let is_dep_typescript_enabled = dep_project.config.toolchain.is_typescript_enabled();

        // Update dependencies within this project's `package.json`.
        // Only add if the dependent project has a `package.json`,
        // and this `package.json` has not already declared the dep.
        if node_config.sync_project_workspace_dependencies {
            let format = &node_config.dependency_version_format;

            if let Some(dep_package_json) = PackageJson::read(&dep_project.root)? {
                if let Some(dep_package_name) = &dep_package_json.name {
                    let version_prefix = format.get_prefix();
                    let dep_package_version = dep_package_json.version.unwrap_or_default();
                    let dep_version = match format {
                        NodeVersionFormat::File | NodeVersionFormat::Link => {
                            format!(
                                "{}{}",
                                version_prefix,
                                path::to_virtual_string(&dep_relative_path)?
                            )
                        }
                        NodeVersionFormat::Version
                        | NodeVersionFormat::VersionCaret
                        | NodeVersionFormat::VersionTilde => {
                            format!("{version_prefix}{dep_package_version}")
                        }
                        _ => version_prefix,
                    };

                    match dep_cfg.scope {
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
                                    semver::extract_major_version(&dep_package_version)
                                ),
                            );
                        }
                    }

                    debug!(
                        target: LOG_TARGET,
                        "Syncing {} as a dependency to {}'s {}",
                        color::id(&dep_project.id),
                        color::id(&project.id),
                        color::file(NPM.manifest)
                    );
                }
            }
        }

        if let Some(typescript_config) = &typescript_config {
            // Update `references` within this project's `tsconfig.json`.
            // Only add if the dependent project has a `tsconfig.json`,
            // and this `tsconfig.json` has not already declared the dep.
            if is_project_typescript_enabled
                && is_dep_typescript_enabled
                && dep_project
                    .root
                    .join(&typescript_config.project_config_file_name)
                    .exists()
            {
                tsconfig_project_refs.insert(path::to_virtual_string(&dep_relative_path)?);

                debug!(
                    target: LOG_TARGET,
                    "Syncing {} as a project reference to {}'s {}",
                    color::id(&dep_project.id),
                    color::id(&project.id),
                    color::file(&typescript_config.project_config_file_name)
                );
            }

            // Map the depended on reference as a `paths` alias using
            // the dep's `package.json` name.
            if is_project_typescript_enabled && is_dep_typescript_enabled {
                if let Some(dep_package_json) = PackageJson::read(&dep_project.root)? {
                    if let Some(dep_package_name) = &dep_package_json.name {
                        for index in ["src/index.ts", "src/index.tsx", "index.ts", "index.tsx"] {
                            if dep_project.root.join(index).exists() {
                                tsconfig_paths.insert(
                                    dep_package_name.clone(),
                                    vec![path::to_virtual_string(dep_relative_path.join(index))?],
                                );

                                tsconfig_paths.insert(
                                    format!("{dep_package_name}/*"),
                                    vec![path::to_virtual_string(dep_relative_path.join(
                                        if index.starts_with("src") {
                                            "src/*"
                                        } else {
                                            "*"
                                        },
                                    ))?],
                                );

                                debug!(
                                    target: LOG_TARGET,
                                    "Syncing {} as a import path alias to {}'s {}",
                                    color::id(&dep_project.id),
                                    color::id(&project.id),
                                    color::file(&typescript_config.project_config_file_name)
                                );

                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Sync to the project's `package.json`
    if !package_prod_deps.is_empty()
        || !package_dev_deps.is_empty()
        || !package_peer_deps.is_empty()
    {
        PackageJson::sync(&project.root, |package_json| {
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
                mutated_project_files = true;
            }

            Ok(mutated_package)
        })?;
    }

    // Sync the project and root `tsconfig.json`
    if let Some(typescript_config) = &typescript_config {
        if moon_typescript_platform::sync_project(
            project,
            typescript_config,
            workspace_root,
            tsconfig_paths,
            tsconfig_project_refs,
        )? {
            mutated_project_files = true;
        }
    }

    Ok(mutated_project_files)
}
