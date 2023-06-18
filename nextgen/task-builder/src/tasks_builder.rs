#![allow(dead_code)]

use moon_common::{color, Id};
use moon_config::{InheritedTasksConfig, ProjectWorkspaceInheritedTasksConfig, TaskConfig};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::path::Path;
use tracing::debug;

pub struct TasksBuilder<'proj> {
    project_root: &'proj Path,
    workspace_root: &'proj Path,

    global_tasks: BTreeMap<&'proj Id, &'proj TaskConfig>,
}

impl<'proj> TasksBuilder<'proj> {
    pub fn new(project_root: &'proj Path, workspace_root: &'proj Path) -> Self {
        Self {
            project_root,
            workspace_root,
            global_tasks: BTreeMap::new(),
        }
    }

    pub fn inherit_global_tasks(
        &mut self,
        global_config: &'proj InheritedTasksConfig,
        global_filters: Option<&'proj ProjectWorkspaceInheritedTasksConfig>,
    ) -> &mut Self {
        let mut include_all = true;
        let mut include_set = FxHashSet::default();
        let mut exclude = vec![];
        let mut rename = FxHashMap::default();

        if let Some(filters) = global_filters {
            exclude.extend(&filters.exclude);
            rename.extend(&filters.rename);

            if let Some(include_config) = &filters.include {
                include_all = false;
                include_set.extend(include_config);
            }
        }

        debug!("Inheriting and filtering global tasks");

        for (task_id, task_config) in &global_config.tasks {
            // None = Include all
            // [] = Include none
            // ["a"] = Include "a"
            if !include_all {
                if include_set.is_empty() {
                    debug!("Not inheriting global tasks, empty include filter");

                    break;
                } else if !include_set.contains(task_id) {
                    debug!(
                        "Not inheriting global task {}, not included",
                        color::id(task_id)
                    );

                    continue;
                }
            }

            // None, [] = Exclude none
            // ["a"] = Exclude "a"
            if !exclude.is_empty() && exclude.contains(&&task_id) {
                debug!(
                    "Not inheriting global task {}, excluded",
                    color::id(task_id)
                );

                continue;
            }

            let task_key = if let Some(renamed_task_id) = rename.get(task_id) {
                debug!(
                    "Inheriting global task {} and renaming to {}",
                    color::id(task_id),
                    color::id(renamed_task_id)
                );

                renamed_task_id
            } else {
                debug!("Inheriting global task {}", color::id(task_id));

                task_id
            };

            self.global_tasks.insert(task_key, task_config);
        }

        self
    }
}
