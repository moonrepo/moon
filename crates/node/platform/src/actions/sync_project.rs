use moon_common::Id;
use moon_config::{DependencyScope, NodeConfig, TypeScriptConfig};
use moon_javascript_platform::JavaScriptSyncer;
use moon_logger::debug;
use moon_project::Project;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_styles::color;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub async fn sync_project(
    project: &Project,
    dependencies: &FxHashMap<Id, Arc<Project>>,
    workspace_root: &Path,
    node_config: &NodeConfig,
    typescript_config: &Option<TypeScriptConfig>,
) -> miette::Result<bool> {
    let mut mutated_project_files = false;

    if JavaScriptSyncer::for_node(project, node_config).sync(dependencies)? {
        mutated_project_files = true;
    }

    let is_project_typescript_enabled = project.config.toolchain.is_typescript_enabled();

    // Sync each dependency to `tsconfig.json` and `package.json`
    let mut tsconfig_project_refs: FxHashSet<PathBuf> = FxHashSet::default();

    for (dep_id, dep_cfg) in &project.dependencies {
        let Some(dep_project) = dependencies.get(dep_id) else {
            continue;
        };

        if dep_project.is_root_level() || matches!(dep_cfg.scope, DependencyScope::Root) {
            continue;
        }

        let is_dep_typescript_enabled = dep_project.config.toolchain.is_typescript_enabled();

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
                tsconfig_project_refs.insert(dep_project.root.clone());

                debug!(
                    "Syncing {} as a project reference to {}'s {}",
                    color::id(&dep_project.id),
                    color::id(&project.id),
                    color::file(&typescript_config.project_config_file_name)
                );
            }
        }
    }

    // Sync the project and root `tsconfig.json`
    if let Some(typescript_config) = &typescript_config {
        if moon_typescript_platform::sync_project(
            project,
            typescript_config,
            workspace_root,
            tsconfig_project_refs,
        )? {
            mutated_project_files = true;
        }
    }

    Ok(mutated_project_files)
}
