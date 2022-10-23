use moon_action::{Action, ActionContext, ActionStatus};
use moon_config::{DependencyScope, NodeVersionFormat, TypeScriptConfig};
use moon_lang_node::tsconfig::CompilerOptionsPaths;
use moon_lang_node::{package::PackageJson, tsconfig::TsConfigJson, NPM};
use moon_logger::{color, debug};
use moon_project::Project;
use moon_utils::{fs, is_ci, path, semver, string_vec};
use moon_workspace::{Workspace, WorkspaceError};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:platform-node:sync-project";

// Automatically create missing config files when we are syncing project references.
#[track_caller]
async fn create_missing_tsconfig(
    project: &Project,
    typescript_config: &TypeScriptConfig,
    workspace_root: &Path,
) -> Result<bool, WorkspaceError> {
    let tsconfig_path = project
        .root
        .join(&typescript_config.project_config_file_name);

    if tsconfig_path.exists() {
        return Ok(false);
    }

    let tsconfig_options_path =
        workspace_root.join(&typescript_config.root_options_config_file_name);

    let json = TsConfigJson {
        extends: Some(path::to_virtual_string(
            path::relative_from(&tsconfig_options_path, &project.root).unwrap(),
        )?),
        include: Some(string_vec!["**/*"]),
        references: Some(vec![]),
        path: tsconfig_path.clone(),
        ..TsConfigJson::default()
    };

    fs::write_json(&tsconfig_path, &json, true).await?;

    Ok(true)
}

// Sync projects references to the root `tsconfig.json`.
fn sync_root_tsconfig(
    tsconfig: &mut TsConfigJson,
    typescript_config: &TypeScriptConfig,
    project: &Project,
) -> bool {
    if project
        .root
        .join(&typescript_config.project_config_file_name)
        .exists()
        && tsconfig.add_project_ref(&project.source, &typescript_config.project_config_file_name)
    {
        debug!(
            target: LOG_TARGET,
            "Syncing {} as a project reference to the root {}",
            color::id(&project.id),
            color::file(&typescript_config.root_config_file_name)
        );

        return true;
    }

    false
}

pub async fn sync_project(
    _action: &mut Action,
    _context: &ActionContext,
    workspace: Arc<RwLock<Workspace>>,
    project: &Project,
) -> Result<ActionStatus, WorkspaceError> {
    let mut mutated_files = false;
    let workspace = workspace.read().await;
    let node = workspace.toolchain.node.get()?;
    let is_project_typescript_enabled = project.config.workspace.typescript;

    // Sync each dependency to `tsconfig.json` and `package.json`
    let mut package_prod_deps: BTreeMap<String, String> = BTreeMap::new();
    let mut package_peer_deps: BTreeMap<String, String> = BTreeMap::new();
    let mut package_dev_deps: BTreeMap<String, String> = BTreeMap::new();
    let mut tsconfig_project_refs: HashSet<String> = HashSet::new();
    let mut tsconfig_paths: CompilerOptionsPaths = BTreeMap::new();

    for (dep_id, dep_cfg) in &project.dependencies {
        // TODO
        let dep_project = Project::default(); // workspace.projects.load(dep_id)?;
        let dep_relative_path =
            path::relative_from(&dep_project.root, &project.root).unwrap_or_default();
        let is_dep_typescript_enabled = dep_project.config.workspace.typescript;

        // Update dependencies within this project's `package.json`.
        // Only add if the dependent project has a `package.json`,
        // and this `package.json` has not already declared the dep.
        if node.config.sync_project_workspace_dependencies {
            let format = &node.config.dependency_version_format;

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
                            format!("{}{}", version_prefix, dep_package_version)
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
                        color::file(NPM.manifest_filename)
                    );
                }
            }
        }

        if let Some(typescript_config) = &workspace.config.typescript {
            // Update `references` within this project's `tsconfig.json`.
            // Only add if the dependent project has a `tsconfig.json`,
            // and this `tsconfig.json` has not already declared the dep.
            if is_project_typescript_enabled
                && is_dep_typescript_enabled
                && typescript_config.sync_project_references
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
            if is_project_typescript_enabled
                && is_dep_typescript_enabled
                && typescript_config.sync_project_references_to_paths
            {
                if let Some(dep_package_json) = PackageJson::read(&dep_project.root)? {
                    if let Some(dep_package_name) = &dep_package_json.name {
                        for index in ["src/index.ts", "src/index.tsx", "index.ts", "index.tsx"] {
                            if dep_project.root.join(&index).exists() {
                                tsconfig_paths.insert(
                                    dep_package_name.clone(),
                                    vec![path::to_virtual_string(dep_relative_path.join(&index))?],
                                );

                                tsconfig_paths.insert(
                                    format!("{}/*", dep_package_name),
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
            for (name, version) in package_prod_deps {
                if package_json.add_dependency(&name, &version, true) {
                    mutated_files = true;
                }
            }

            for (name, version) in package_dev_deps {
                if package_json.add_dev_dependency(&name, &version, true) {
                    mutated_files = true;
                }
            }

            for (name, version) in package_peer_deps {
                if package_json.add_peer_dependency(&name, &version, true) {
                    mutated_files = true;
                }
            }

            Ok(())
        })?;
    }

    if let Some(typescript_config) = &workspace.config.typescript {
        // Auto-create a `tsconfig.json` if configured and applicable
        if is_project_typescript_enabled
            && typescript_config.sync_project_references
            && typescript_config.create_missing_config
            && !project
                .root
                .join(&typescript_config.project_config_file_name)
                .exists()
        {
            create_missing_tsconfig(project, typescript_config, &workspace.root).await?;
        }

        // Sync to the project's `tsconfig.json`
        if is_project_typescript_enabled {
            TsConfigJson::sync_with_name(
                &project.root,
                &typescript_config.project_config_file_name,
                |tsconfig_json| {
                    // Project references
                    if !tsconfig_project_refs.is_empty() {
                        for ref_path in tsconfig_project_refs {
                            if tsconfig_json.add_project_ref(
                                &ref_path,
                                &typescript_config.project_config_file_name,
                            ) {
                                mutated_files = true;
                            }
                        }
                    }

                    // Out dir
                    if typescript_config.route_out_dir_to_cache {
                        let cache_route = workspace.cache.dir.join("types").join(&project.source);

                        tsconfig_json.update_compiler_options().out_dir =
                            Some(path::to_virtual_string(
                                path::relative_from(&cache_route, &project.root).unwrap(),
                            )?);
                    }

                    // Paths
                    if typescript_config.sync_project_references_to_paths
                        && !tsconfig_paths.is_empty()
                    {
                        tsconfig_json
                            .update_compiler_options()
                            .update_paths()
                            .extend(tsconfig_paths);
                    }

                    Ok(())
                },
            )?;
        }

        // Sync to the root `tsconfig.json`
        if is_project_typescript_enabled && typescript_config.sync_project_references {
            TsConfigJson::sync_with_name(
                &workspace.root,
                &typescript_config.root_config_file_name,
                |tsconfig_json| {
                    if sync_root_tsconfig(tsconfig_json, typescript_config, project) {
                        mutated_files = true;
                    }

                    Ok(())
                },
            )?;
        }
    }

    if mutated_files {
        // If files have been modified in CI, we should update the status to warning,
        // as these modifications should be committed to the repo.
        if is_ci() {
            return Ok(ActionStatus::Invalid);
        } else {
            return Ok(ActionStatus::Passed);
        }
    }

    Ok(ActionStatus::Skipped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_config::GlobalProjectConfig;
    use moon_utils::test::create_sandbox;

    #[tokio::test]
    async fn creates_tsconfig() {
        let fixture = create_sandbox("cases");

        let project = Project::new(
            "deps-a",
            "deps-a",
            fixture.path(),
            &GlobalProjectConfig::default(),
        )
        .unwrap();

        let tsconfig_path = project.root.join("tsconfig.json");

        assert!(!tsconfig_path.exists());

        create_missing_tsconfig(&project, &TypeScriptConfig::default(), fixture.path())
            .await
            .unwrap();

        assert!(tsconfig_path.exists());

        let tsconfig = TsConfigJson::read(tsconfig_path).unwrap().unwrap();

        assert_eq!(
            tsconfig.extends,
            Some("../tsconfig.options.json".to_owned())
        );
        assert_eq!(tsconfig.include, Some(string_vec!["**/*"]));
    }

    #[tokio::test]
    async fn creates_tsconfig_with_custom_settings() {
        let fixture = create_sandbox("cases");

        let project = Project::new(
            "deps-a",
            "deps-a",
            fixture.path(),
            &GlobalProjectConfig::default(),
        )
        .unwrap();

        let tsconfig_path = project.root.join("tsconfig.ref.json");

        assert!(!tsconfig_path.exists());

        create_missing_tsconfig(
            &project,
            &TypeScriptConfig {
                project_config_file_name: "tsconfig.ref.json".to_string(),
                root_options_config_file_name: "tsconfig.base.json".to_string(),
                ..TypeScriptConfig::default()
            },
            fixture.path(),
        )
        .await
        .unwrap();

        assert!(tsconfig_path.exists());

        let tsconfig = TsConfigJson::read_with_name(&project.root, "tsconfig.ref.json")
            .unwrap()
            .unwrap();

        assert_eq!(tsconfig.extends, Some("../tsconfig.base.json".to_owned()));
        assert_eq!(tsconfig.include, Some(string_vec!["**/*"]));
    }

    #[tokio::test]
    async fn doesnt_create_if_a_config_exists() {
        let fixture = create_sandbox("cases");

        let project = Project::new(
            "deps-b",
            "deps-b",
            fixture.path(),
            &GlobalProjectConfig::default(),
        )
        .unwrap();

        let tsconfig_path = project.root.join("tsconfig.json");

        assert!(tsconfig_path.exists());

        let created =
            create_missing_tsconfig(&project, &TypeScriptConfig::default(), fixture.path())
                .await
                .unwrap();

        assert!(!created);
    }
}
