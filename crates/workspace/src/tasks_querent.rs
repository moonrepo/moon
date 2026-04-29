use crate::projects_builder::ProjectBuildData;
use crate::tasks_builder::TaskBuildData;
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
        task_id: &str,
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
                let other_project_id = target.get_project_id().ok()?;
                let other_task_id = target.get_task_id().ok()?;

                if other_task_id == task_id && project_ids.iter().any(|id| id == other_project_id) {
                    Some((target, &data.options))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(results)
    }
}

pub struct WorkspaceTasksQuerent<'a> {
    pub aliases_to_ids: &'a FxHashMap<String, Id>,
    pub ids_to_target_options: &'a FxHashMap<Id, FxHashMap<Target, TaskOptions>>,
    pub tags_to_ids: &'a FxHashMap<Id, Vec<Id>>,
}

impl<'a> TasksQuerent for WorkspaceTasksQuerent<'a> {
    fn query_projects_by_tag(&self, tag: &str) -> miette::Result<Vec<&Id>> {
        Ok(self
            .tags_to_ids
            .get(tag)
            .map(|list| list.iter().collect())
            .unwrap_or_default())
    }

    fn query_tasks(
        &self,
        project_ids: Vec<&Id>,
        task_id: &str,
    ) -> miette::Result<Vec<(&Target, &TaskOptions)>> {
        let mut list = vec![];

        for project_id in project_ids {
            if let Some(tasks) = self.ids_to_target_options.get(project_id) {
                for (target, options) in tasks {
                    if target.get_task_id()? == task_id {
                        list.push((target, options));
                    }
                }
            }
        }

        Ok(list)
    }
}
