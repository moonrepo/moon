use crate::inherited_tasks_config::*;
use crate::project_config::{LayerType, StackType};
use crate::shapes::{Input, OneOrMany};
use miette::IntoDiagnostic;
use moon_common::path::PathExt;
use moon_common::{
    Id, color,
    path::{WorkspaceRelativePathBuf, standardize_separators},
};
use rustc_hash::FxHashMap;
use schematic::schema::indexmap::IndexMap;
use schematic::{Config, ConfigError, PartialConfig};
use std::path::Path;

#[derive(Debug, Default)]
pub struct InheritedTasksEntry {
    pub input: WorkspaceRelativePathBuf,
    pub config: PartialInheritedTasksConfig,
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
            config,
        });

        Ok(())
    }

    pub fn get_inherited_config(
        &self,
        root: &Path,
        toolchains: &[Id],
        stack: &StackType,
        layer: &LayerType,
        tags: &[Id],
    ) -> miette::Result<InheritedTasksResult> {
        let mut partial_config = PartialInheritedTasksConfig::default();
        let mut layers = IndexMap::default();
        let mut task_layers = FxHashMap::<String, Vec<String>>::default();
        let mut lookup_order = vec![];

        #[allow(clippy::let_unit_value)]
        let context = ();

        for config_entry in
            self.match_inherited_configs_in_order(root, toolchains, stack, layer, tags)
        {
            let source_path = standardize_separators(config_entry.input.as_str());
            let mut config = config_entry.config.clone();

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

            layers.insert(source_path.clone(), config.clone());
            partial_config.merge(&context, config)?;
            lookup_order.push(source_path);
        }

        let full_config = partial_config.finalize(&context)?;

        full_config
            .validate(&context, true)
            .map_err(|error| match error {
                ConfigError::Validator { error, .. } => ConfigError::Validator {
                    location: format!(
                        "inherited tasks ({}, {}, {})",
                        toolchains.join(", "),
                        stack,
                        layer
                    ),
                    error,
                    help: Some(color::muted_light("https://moonrepo.dev/docs/config/tasks")),
                },
                _ => error,
            })?;

        let result = InheritedTasksResult {
            config: InheritedTasksConfig::from_partial(full_config),
            layers,
            order: lookup_order,
            task_layers,
        };

        Ok(result)
    }

    pub fn match_inherited_configs_in_order(
        &self,
        root: &Path,
        toolchains: &[Id],
        stack: &StackType,
        layer: &LayerType,
        tags: &[Id],
    ) -> Vec<&InheritedTasksEntry> {
        let mut entries = self
            .configs
            .iter()
            .filter(|entry| {
                match &entry.config.inherited_by {
                    Some(by) => by.matches(root, toolchains, stack, layer, tags),
                    // If no inherited by setting, then it's inherited by all!
                    None => true,
                }
            })
            .collect::<Vec<_>>();

        entries.sort_by(|a, d| {
            let a_order = a.config.inherited_by.as_ref().map_or(0, |by| by.order());
            let d_order = d.config.inherited_by.as_ref().map_or(0, |by| by.order());

            a_order.cmp(&d_order)
        });

        entries
    }
}
