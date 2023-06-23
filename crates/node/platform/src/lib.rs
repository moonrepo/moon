pub mod actions;
mod platform;
mod target_hasher;
pub mod task;

pub use platform::NodePlatform;
pub use target_hasher::NodeTargetHasher;

use moon_common::Id;
use moon_config::PartialTaskConfig;
use moon_node_lang::PackageJson;
use std::collections::BTreeMap;
use task::ScriptParser;

pub fn create_tasks_from_scripts(
    project_id: &str,
    package_json: &mut PackageJson,
) -> miette::Result<BTreeMap<Id, PartialTaskConfig>> {
    let mut parser = ScriptParser::new(project_id);

    parser.parse_scripts(package_json)?;
    parser.update_package(package_json)?;

    Ok(parser.tasks)
}

pub fn infer_tasks_from_scripts(
    project_id: &str,
    package_json: &PackageJson,
) -> miette::Result<BTreeMap<Id, PartialTaskConfig>> {
    let mut parser = ScriptParser::new(project_id);

    parser.infer_scripts(package_json)?;

    Ok(parser.tasks)
}
