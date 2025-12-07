use crate::inherited_tasks_config::*;
use crate::shapes::{Input, OneOrMany};
use miette::IntoDiagnostic;
use moon_common::{
    color,
    path::{PathExt, WorkspaceRelativePathBuf, standardize_separators},
};
use rustc_hash::FxHashMap;
use schematic::schema::indexmap::IndexMap;
use schematic::{Config, ConfigError, PartialConfig};
use std::path::Path;

#[derive(Debug, Default)]
pub struct InheritedTasksEntry {
    pub input: WorkspaceRelativePathBuf,
    pub config: InheritedTasksConfig,
    #[deprecated]
    pub partial_config: PartialInheritedTasksConfig,
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
        config: PartialInheritedTasksConfig,
    ) -> miette::Result<()> {
        self.configs.push(InheritedTasksEntry {
            input: config_path.relative_to(workspace_root).into_diagnostic()?,
            config: InheritedTasksConfig::from_partial(config.clone()),
            partial_config: config,
        });

        Ok(())
    }

    pub fn get_inherited_config(&self, input: InheritFor) -> miette::Result<InheritedTasks> {
        let mut partial_config = PartialInheritedTasksConfig::default();
        let mut configs = IndexMap::default();
        let mut layers = IndexMap::default();
        let mut task_layers = FxHashMap::<String, Vec<String>>::default();
        let mut lookup_order = vec![];

        #[allow(clippy::let_unit_value)]
        let context = ();

        for config_entry in self.match_inherited_configs_in_order(input) {
            let source_path = standardize_separators(config_entry.input.as_str());
            let mut config = config_entry.partial_config.clone();

            if let Some(tasks) = &mut config.tasks {
                let default_toolchain = config
                    .inherited_by
                    .as_ref()
                    .and_then(|by| by.default_toolchain());

                for (task_id, task) in tasks.iter_mut() {
                    // Automatically set this source as an input
                    task.global_inputs
                        .get_or_insert(vec![])
                        .push(Input::parse(format!("/{source_path}"))?);

                    // Automatically set the toolchain
                    if task.toolchains.is_none()
                        && let Some(toolchain) = &default_toolchain
                    {
                        task.toolchains = Some(OneOrMany::One(toolchain.to_owned()));
                    }

                    // Keep track of what layers a task inherited
                    task_layers
                        .entry(task_id.to_string())
                        .or_default()
                        .push(source_path.clone());
                }
            }

            configs.insert(
                source_path.clone(),
                InheritedTasksConfig::from_partial(config.clone()),
            );
            layers.insert(source_path.clone(), config.clone());
            partial_config.merge(&context, config)?;
            lookup_order.push(source_path);
        }

        let full_config = partial_config.finalize(&context)?;

        full_config
            .validate(&context, true)
            .map_err(|error| match error {
                ConfigError::Validator { error, .. } => ConfigError::Validator {
                    location: format!("inherited tasks ({})", lookup_order.join(", ")),
                    error,
                    help: Some(color::muted_light("https://moonrepo.dev/docs/config/tasks")),
                },
                _ => error,
            })?;

        let result = InheritedTasks {
            configs,
            config: InheritedTasksConfig::from_partial(full_config),
            layers,
            order: lookup_order,
            task_layers,
        };

        Ok(result)
    }

    pub fn match_inherited_configs_in_order(&self, input: InheritFor) -> Vec<&InheritedTasksEntry> {
        let mut entries = self
            .configs
            .iter()
            .filter(|entry| {
                match &entry.partial_config.inherited_by {
                    Some(by) => by.matches(&input),
                    // If no `inheritedBy` setting, then it's inherited by all!
                    None => true,
                }
            })
            .collect::<Vec<_>>();

        entries.sort_by_key(|entry| {
            (
                entry
                    .partial_config
                    .inherited_by
                    .as_ref()
                    .map_or(0, |by| by.order()),
                entry.input.as_str(),
            )
        });

        entries
    }
}
