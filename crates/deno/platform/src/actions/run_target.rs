use std::collections::BTreeMap;

use crate::target_hasher::DenoTargetHasher;
use moon_config::{HasherConfig, HasherOptimization};
use moon_deno_lang::{load_lockfile_dependencies, DENO_DEPS};
use moon_deno_tool::DenoTool;
use moon_project::Project;
use moon_tool::ToolError;
use rustc_hash::FxHashMap;

pub async fn create_target_hasher(
    tool: Option<&DenoTool>,
    project: &Project,
    hasher_config: &HasherConfig,
) -> Result<DenoTargetHasher, ToolError> {
    let mut hasher = DenoTargetHasher::new(None);

    let resolved_dependencies =
        if matches!(hasher_config.optimization, HasherOptimization::Accuracy) && tool.is_some() {
            load_lockfile_dependencies(project.root.join(DENO_DEPS.lockfile))?
        } else {
            FxHashMap::default()
        };

    hasher.hash_deps(BTreeMap::from_iter(resolved_dependencies));

    // Hash deno.json?

    Ok(hasher)
}
