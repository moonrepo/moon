use moon_config::{DenoConfig, TypeScriptConfig};
use moon_project::{Project, ProjectError};
use moon_typescript_lang::tsconfig::CompilerOptionsPaths;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::path::Path;

// const LOG_TARGET: &str = "moon:deno-platform:sync-project";

pub async fn sync_project(
    project: &Project,
    _dependencies: &FxHashMap<String, &Project>,
    workspace_root: &Path,
    _deno_config: &DenoConfig,
    typescript_config: &Option<TypeScriptConfig>,
) -> Result<bool, ProjectError> {
    let mut mutated_project_files = false;
    let tsconfig_project_refs: FxHashSet<String> = FxHashSet::default();
    let tsconfig_paths: CompilerOptionsPaths = BTreeMap::new();

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
