use moon_common::Id;
use moon_config::{NodeConfig, TypeScriptConfig};
use moon_javascript_platform::JavaScriptSyncer;
use moon_project::Project;
use moon_typescript_platform::TypeScriptSyncer;
use rustc_hash::FxHashMap;
use std::path::Path;
use std::sync::Arc;

pub async fn sync_project(
    project: &Project,
    dependencies: &FxHashMap<Id, Arc<Project>>,
    workspace_root: &Path,
    node_config: &NodeConfig,
    typescript_config: &Option<TypeScriptConfig>,
) -> miette::Result<bool> {
    let mut mutated = false;

    if JavaScriptSyncer::for_node(project, node_config).sync(dependencies)? {
        mutated = true;
    }

    if let Some(config) = &typescript_config {
        if TypeScriptSyncer::new(project, config, workspace_root).sync(dependencies)? {
            mutated = true;
        }
    }

    Ok(mutated)
}
