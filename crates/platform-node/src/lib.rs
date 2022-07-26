mod hasher;
pub mod task;

pub use hasher::NodeTargetHasher;
use moon_lang_node::package::PackageJson;
use moon_task::TaskError;
use std::path::Path;
use task::{ScriptParser, TasksMap};

pub fn create_tasks_from_scripts(
    project_id: &str,
    package_json: &mut PackageJson,
) -> Result<TasksMap, TaskError> {
    let mut parser = ScriptParser::new(project_id);

    parser.parse_scripts(package_json)?;
    parser.update_package(package_json)?;

    Ok(parser.tasks)
}

pub fn infer_tasks_from_scripts(
    project_id: &str,
    package_json: &PackageJson,
) -> Result<TasksMap, TaskError> {
    let mut parser = ScriptParser::new(project_id);

    parser.infer_scripts(package_json)?;

    Ok(parser.tasks)
}

pub fn infer_tasks(project_id: &str, project_root: &Path) -> Result<Option<TasksMap>, TaskError> {
    if let Some(package_json) = PackageJson::read(project_root)? {
        return Ok(Some(infer_tasks_from_scripts(project_id, &package_json)?));
    }

    Ok(None)
}
