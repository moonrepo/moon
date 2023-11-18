use moon_common::Id;
use moon_config::{DenoConfig, TypeScriptConfig};
use moon_project::Project;
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// const LOG_TARGET: &str = "moon:deno-platform:sync-project";

pub async fn sync_project(
    project: &Project,
    _dependencies: &FxHashMap<Id, Arc<Project>>,
    workspace_root: &Path,
    _deno_config: &DenoConfig,
    typescript_config: &Option<TypeScriptConfig>,
) -> miette::Result<bool> {
    let mut mutated_project_files = false;
    let tsconfig_project_refs: FxHashSet<PathBuf> = FxHashSet::default();

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
