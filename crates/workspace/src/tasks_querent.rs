use crate::build_data::{ProjectBuildData, TaskBuildData};
use moon_common::Id;
use moon_task::{Target, TaskOptions};
use moon_task_builder::TasksQuerent;
use rustc_hash::FxHashMap;

pub struct WorkspaceBuilderTasksQuerent<'builder> {
    pub project_data: &'builder FxHashMap<Id, ProjectBuildData>,
    pub projects_by_tag: &'builder FxHashMap<Id, Vec<Id>>,
    pub task_data: &'builder FxHashMap<Target, TaskBuildData>,
}

impl TasksQuerent for WorkspaceBuilderTasksQuerent<'_> {
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
        // May be an alias!
        let project_ids = project_ids
            .iter()
            .map(|id| ProjectBuildData::resolve_id(id, self.project_data))
            .collect::<Vec<_>>();

        let results = self
            .task_data
            .iter()
            .filter_map(|(target, data)| {
                let project_id = target.get_project_id()?;

                if &target.task_id == task_id && project_ids.contains(project_id) {
                    Some((target, &data.options))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(results)
    }
}
