use crate::inherited_tasks_config::*;
use crate::shapes::{Input, OneOrMany};
use miette::IntoDiagnostic;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf, standardize_separators};
use rustc_hash::FxHashMap;
use schematic::schema::indexmap::IndexMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct InheritedTasksEntry {
    pub input: WorkspaceRelativePathBuf,
    pub config: InheritedTasksConfig,
}

#[derive(Debug, Default)]
pub struct InheritedTasksManager {
    pub configs: Vec<InheritedTasksEntry>,
}

impl InheritedTasksManager {
    pub fn add_config(
        &mut self,
        workspace_root: &Path,
        config_path: &Path,
        config: InheritedTasksConfig,
    ) -> miette::Result<()> {
        self.configs.push(InheritedTasksEntry {
            input: config_path.relative_to(workspace_root).into_diagnostic()?,
            config,
        });

        Ok(())
    }

    pub fn get_inherited_config(&self, input: InheritFor) -> miette::Result<InheritedTasks> {
        let mut configs = IndexMap::default();
        let mut layers = FxHashMap::<String, Vec<String>>::default();

        for config_entry in self.match_inherited_configs_in_order(input) {
            let source_path = standardize_separators(config_entry.input.as_str());
            let mut config = config_entry.config.clone();

            let default_toolchain = config
                .inherited_by
                .as_ref()
                .and_then(|by| by.default_toolchain());

            for (task_id, task) in config.tasks.iter_mut() {
                // Automatically set this source as an input
                task.global_inputs
                    .push(Input::parse(format!("/{source_path}"))?);

                // Automatically set the toolchain
                if task.toolchains.is_none()
                    && let Some(toolchain) = &default_toolchain
                {
                    task.toolchains = Some(OneOrMany::One(toolchain.to_owned()));
                }

                // Keep track of what layers a task inherited
                layers
                    .entry(task_id.to_string())
                    .or_default()
                    .push(source_path.clone());
            }

            configs.insert(source_path, config);
        }

        Ok(InheritedTasks { configs, layers })
    }

    pub fn match_inherited_configs_in_order(&self, input: InheritFor) -> Vec<&InheritedTasksEntry> {
        let mut entries = self
            .configs
            .iter()
            .filter(|entry| {
                match &entry.config.inherited_by {
                    Some(by) => by.matches(&input),
                    // If no `inheritedBy` setting, then it's inherited by all!
                    None => true,
                }
            })
            .collect::<Vec<_>>();

        entries.sort_by_key(|entry| {
            (
                entry
                    .config
                    .inherited_by
                    .as_ref()
                    .map_or(0, |by| by.order()),
                entry.input.as_str(),
            )
        });

        entries
    }
}
