use moon_common::Id;
use moon_task::{Target, TargetTaskScope, TaskOptions};
use moon_task_builder::TasksQuerent;
use rustc_hash::FxHashMap;

pub struct WorkspaceTasksQuerent<'a> {
    pub aliases_to_ids: &'a FxHashMap<String, Id>,
    pub ids_to_target_options: &'a FxHashMap<Id, FxHashMap<Target, TaskOptions>>,
    pub tags_to_ids: &'a FxHashMap<Id, Vec<Id>>,
    pub tags_to_targets: &'a FxHashMap<Id, Vec<Target>>,
    pub target_to_has_outputs: &'a FxHashMap<Target, bool>,
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
        task_scope: (TargetTaskScope, &str),
    ) -> miette::Result<Vec<(&Target, &TaskOptions)>> {
        let mut list = vec![];

        for project_id in project_ids {
            // May be an alias!
            let project_id = self
                .aliases_to_ids
                .get(project_id.as_str())
                .unwrap_or(project_id);

            if let Some(tasks) = self.ids_to_target_options.get(project_id) {
                for (target, options) in tasks {
                    match task_scope {
                        (TargetTaskScope::Id, task_id) => {
                            if target.get_task_id()? == task_id {
                                list.push((target, options));
                            }
                        }
                        (TargetTaskScope::Tag, tag_id) => {
                            if self
                                .tags_to_targets
                                .get(tag_id)
                                .is_some_and(|targets| targets.contains(target))
                            {
                                list.push((target, options));
                            }
                        }
                    }
                }
            }
        }

        Ok(list)
    }

    fn query_task_has_outputs(&self, target: &Target) -> bool {
        self.target_to_has_outputs
            .get(target)
            .copied()
            .unwrap_or(false)
    }
}
