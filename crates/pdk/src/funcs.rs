use extism_pdk::*;
use moon_common::Id;
use moon_pdk_api::AnyResult;
use moon_project::Project;
use moon_task::{Target, Task};
use rustc_hash::FxHashMap;

#[host_fn]
extern "ExtismHost" {
    fn load_project_by_id(id: String) -> Json<Project>;
    fn load_projects_by_id(ids: Json<Vec<String>>) -> Json<FxHashMap<Id, Project>>;
    fn load_task_by_target(target: String) -> Json<Task>;
    fn load_tasks_by_target(targets: Json<Vec<String>>) -> Json<FxHashMap<Target, Task>>;
}

pub fn load_project(id: impl AsRef<str>) -> AnyResult<Project> {
    let project = unsafe { load_project_by_id(id.as_ref().into())? };

    Ok(project.0)
}

pub fn load_projects<I, V>(ids: I) -> AnyResult<FxHashMap<Id, Project>>
where
    I: IntoIterator<Item = V>,
    V: AsRef<str>,
{
    let projects = unsafe {
        load_projects_by_id(Json::from(
            ids.into_iter()
                .map(|p| p.as_ref().to_owned())
                .collect::<Vec<_>>(),
        ))?
    };

    Ok(projects.0)
}

pub fn load_task(target: impl AsRef<str>) -> AnyResult<Task> {
    let task = unsafe { load_task_by_target(target.as_ref().into())? };

    Ok(task.0)
}

pub fn load_tasks<I, V>(targets: I) -> AnyResult<FxHashMap<Target, Task>>
where
    I: IntoIterator<Item = V>,
    V: AsRef<str>,
{
    let tasks = unsafe {
        load_tasks_by_target(Json::from(
            targets
                .into_iter()
                .map(|p| p.as_ref().to_owned())
                .collect::<Vec<_>>(),
        ))?
    };

    Ok(tasks.0)
}
