pub mod actions;
mod node_platform;
mod target_hash;

pub use node_platform::NodePlatform;
pub use target_hash::NodeTargetHash;

use moon_common::Id;
use moon_config::{NodePackageManager, PartialTaskConfig};
use moon_javascript_platform::ScriptParser;
use moon_node_lang::PackageJsonCache;
use std::collections::BTreeMap;

pub fn create_tasks_from_scripts(
    project_id: &str,
    package_json: &mut PackageJsonCache,
    package_manager: NodePackageManager,
) -> miette::Result<BTreeMap<Id, PartialTaskConfig>> {
    let mut parser = ScriptParser::new(project_id, Id::raw("node"), package_manager);

    parser.parse_scripts(package_json)?;
    parser.update_package(package_json)?;

    Ok(parser.tasks)
}

pub fn infer_tasks_from_scripts(
    project_id: &str,
    package_json: &PackageJsonCache,
    package_manager: NodePackageManager,
) -> miette::Result<BTreeMap<Id, PartialTaskConfig>> {
    let mut parser = ScriptParser::new(project_id, Id::raw("node"), package_manager);

    parser.infer_scripts(package_json)?;

    Ok(parser.tasks)
}
