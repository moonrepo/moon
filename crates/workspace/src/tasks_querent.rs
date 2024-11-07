use crate::build_data::TaskBuildData;
use moon_common::Id;
use moon_task::{Target, TaskOptions};
use moon_task_builder::TasksQuerent;
use rustc_hash::FxHashMap;

pub struct WorkspaceBuilderTasksQuerent<'app> {
    pub projects_by_tag: &'app FxHashMap<Id, Vec<Id>>,
    pub task_data: &'app FxHashMap<Target, TaskBuildData>,
}

impl<'app> TasksQuerent for WorkspaceBuilderTasksQuerent<'app> {
    fn query_projects_by_tag(&self, tag: &str) -> miette::Result<Vec<&Id>> {
        Ok(self
            .projects_by_tag
            .get(tag)
            .map(|list| list.iter().collect())
            .unwrap_or_default())
    }

    fn query_tasks(
        &self,
        project_ids: Vec<&Id>,
        task_id: &Id,
    ) -> miette::Result<Vec<(&Target, &TaskOptions)>> {
        Ok(self
            .task_data
            .iter()
            .filter_map(|(target, data)| {
                if &target.task_id == task_id
                    && target
                        .get_project_id()
                        .is_some_and(|id| project_ids.contains(&id))
                {
                    Some((target, &data.options))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>())
    }
}
