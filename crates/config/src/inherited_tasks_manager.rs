use crate::inherited_tasks_config::*;
use crate::project_config::{LayerType, StackType};
use crate::shapes::{Input, OneOrMany};
use moon_common::{Id, color, path::standardize_separators};
use rustc_hash::{FxHashMap, FxHasher};
use schematic::schema::indexmap::{IndexMap, IndexSet};
use schematic::{Config, ConfigError, PartialConfig};
use std::hash::BuildHasherDefault;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Default)]
pub struct InheritedTasksEntry {
    pub input: PathBuf,
    pub config: PartialInheritedTasksConfig,
}

#[derive(Debug, Default)]
pub struct InheritedTasksManager {
    cache: Arc<RwLock<FxHashMap<String, InheritedTasksResult>>>,

    pub configs: FxHashMap<String, InheritedTasksEntry>,
}

impl InheritedTasksManager {
    pub fn get_lookup_order(
        &self,
        toolchains: &[Id],
        stack: &StackType,
        layer: &LayerType,
        tags: &[Id],
    ) -> Vec<String> {
        let mut lookup: IndexSet<String, BuildHasherDefault<FxHasher>> =
            IndexSet::from_iter(["*".to_string()]);

        // Reverse the order of the toolchains, as the order in the project/task
        // is from most important to least important. But for the configuration,
        // we need the opposite of that, so that the most important is the last
        // layer to be merged in.
        let toolchains = toolchains.iter().rev().collect::<Vec<_>>();

        // Order from least to most specific!

        // frontend
        lookup.insert(format!("{stack}"));

        // frontend-library
        lookup.insert(format!("{stack}-{layer}"));

        for toolchain in &toolchains {
            // node
            lookup.insert(format!("{toolchain}"));
        }

        for toolchain in &toolchains {
            // node-frontend
            lookup.insert(format!("{toolchain}-{stack}"));
        }

        for toolchain in &toolchains {
            // node-library
            lookup.insert(format!("{toolchain}-{layer}"));
        }

        for toolchain in &toolchains {
            // node-frontend-library
            lookup.insert(format!("{toolchain}-{stack}-{layer}"));
        }

        // tag-foo
        for tag in tags {
            lookup.insert(format!("tag-{tag}"));
        }

        lookup
            .into_iter()
            .filter(|item| !item.contains("unknown"))
            .collect()
    }

    pub fn add_config(
        &mut self,
        workspace_root: &Path,
        path: &Path,
        config: PartialInheritedTasksConfig,
    ) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        // TODO: remove after `inheritedBy` implemented for tests
        let name = if name == "all.yml" {
            "*"
        } else if let Some(stripped_name) = name.strip_suffix(".yaml") {
            stripped_name
        } else if let Some(stripped_name) = name.strip_suffix(".yml") {
            stripped_name
        } else if let Some(stripped_name) = name.strip_suffix(".pkl") {
            stripped_name
        } else {
            return;
        };

        self.configs.insert(
            name.to_owned(),
            InheritedTasksEntry {
                input: path.strip_prefix(workspace_root).unwrap().to_path_buf(),
                config,
            },
        );
    }

    pub fn get_inherited_config(
        &self,
        toolchains: &[Id],
        stack: &StackType,
        layer: &LayerType,
        tags: &[Id],
    ) -> miette::Result<InheritedTasksResult> {
        let lookup_order = self.get_lookup_order(toolchains, stack, layer, tags);
        let lookup_key = lookup_order.join(":");

        // Check the cache first in read only mode!
        {
            if let Some(cache) = self.cache.read().unwrap().get(&lookup_key) {
                return Ok(cache.to_owned());
            }
        }

        // Cache the result as this lookup may be the same for a large number of projects,
        // and since this clones constantly, we can avoid a lot of allocations and overhead.
        let mut partial_config = PartialInheritedTasksConfig::default();
        let mut layers = IndexMap::default();
        let mut task_layers = FxHashMap::<String, Vec<String>>::default();

        #[allow(clippy::let_unit_value)]
        let context = ();

        for lookup in &lookup_order {
            if let Some(config_entry) = self.configs.get(lookup) {
                let source_path =
                    standardize_separators(format!("{}", config_entry.input.display()));
                let mut managed_config = config_entry.config.clone();

                // Only modify tasks for `tasks/**/.*` files instead of `tasks.*`,
                // as the latter will be globbed alongside toolchain/workspace configs.
                // We also don't know what toolchain each of the tasks should be yet.
                if let Some(tasks) = &mut managed_config.tasks {
                    for (task_id, task) in tasks.iter_mut() {
                        if lookup != "*" {
                            // Automatically set this source as an input
                            task.global_inputs
                                .get_or_insert(vec![])
                                .push(Input::parse(format!("/{source_path}"))?);

                            // Automatically set the toolchain
                            if task.toolchains.is_none() {
                                task.toolchains = Some(OneOrMany::Many(toolchains.to_owned()));
                            }
                        }

                        // Keep track of what layers a task inherited
                        task_layers
                            .entry(task_id.to_string())
                            .or_default()
                            .push(source_path.clone());
                    }
                }

                layers.insert(source_path, managed_config.clone());
                partial_config.merge(&context, managed_config)?;
            }
        }

        let config = partial_config.finalize(&context)?;

        config
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
            config: InheritedTasksConfig::from_partial(config),
            layers,
            order: lookup_order,
            task_layers,
        };

        self.cache
            .write()
            .unwrap()
            .insert(lookup_key, result.clone());

        Ok(result)
    }
}
