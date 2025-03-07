use moon_common::Id;
use moon_config::NodeConfig;
use moon_javascript_platform::JavaScriptSyncer;
use moon_project::Project;
use rustc_hash::FxHashMap;
use std::sync::Arc;

pub async fn sync_project(
    project: &Project,
    dependencies: &FxHashMap<Id, Arc<Project>>,
    node_config: &NodeConfig,
) -> miette::Result<bool> {
    let mut mutated = false;

    if JavaScriptSyncer::for_node(project, node_config).sync(dependencies)? {
        mutated = true;
    }

    Ok(mutated)
}
