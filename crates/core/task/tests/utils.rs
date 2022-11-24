use moon_config::ProjectConfig;
use moon_project::TaskExpander;
use moon_task::test::{create_file_groups, create_initial_task};
use moon_task::{ResolverData, Task, TaskConfig, TaskError};
use rustc_hash::FxHashMap;
use std::path::Path;

pub fn create_expanded_task(
    workspace_root: &Path,
    project_root: &Path,
    config: Option<TaskConfig>,
) -> Result<Task, TaskError> {
    let mut task = create_initial_task(config);
    let file_groups = create_file_groups();
    let project_config = ProjectConfig::new(project_root);
    let metadata = ResolverData::new(&file_groups, workspace_root, project_root, &project_config);
    let task_expander = TaskExpander::new(&metadata);

    task_expander.expand_env(&mut task)?;
    task_expander.expand_deps(&mut task, "project", &FxHashMap::default())?;
    task_expander.expand_inputs(&mut task)?;
    task_expander.expand_outputs(&mut task)?;
    task_expander.expand_args(&mut task)?;

    Ok(task)
}
