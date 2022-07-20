pub mod task;

use moon_lang_node::package::PackageJson;
use moon_task::TaskError;
use std::path::Path;
use task::TasksMap;

pub use task::create_tasks_from_scripts;

pub fn infer_tasks_from_scripts(
    project_id: &str,
    project_root: &Path,
) -> Result<Option<TasksMap>, TaskError> {
    if let Some(mut package_json) = PackageJson::read(project_root)? {
        return Ok(Some(create_tasks_from_scripts(
            project_id,
            &mut package_json,
        )?));
    }

    Ok(None)
}
