use crate::config_struct;
use crate::project::{
    PartialTaskOptionsConfig, TaskConfig, TaskDependency, TaskOptionsConfig, validate_deps,
};
use crate::project_config::{LayerType, StackType};
use crate::shapes::InputPath;
use moon_common::{Id, cacheable};
use rustc_hash::{FxHashMap, FxHasher};
use schematic::schema::indexmap::{IndexMap, IndexSet};
use schematic::{Config, MergeResult, merge, validate};
use std::collections::BTreeMap;
use std::hash::{BuildHasherDefault, Hash};
use std::path::PathBuf;

#[cfg(feature = "loader")]
use std::{
    path::Path,
    sync::{Arc, RwLock},
};

fn merge_fxhashmap<K, V, C>(
    mut prev: FxHashMap<K, V>,
    next: FxHashMap<K, V>,
    _context: &C,
) -> MergeResult<FxHashMap<K, V>>
where
    K: Eq + Hash,
{
    for (key, value) in next {
        prev.insert(key, value);
    }

    Ok(Some(prev))
}

config_struct!(
    /// Configures tasks and task related settings that'll be inherited by all
    /// matching projects.
    /// Docs: https://moonrepo.dev/docs/config/tasks
    #[derive(Config)]
    pub struct InheritedTasksConfig {
        #[setting(
            default = "https://moonrepo.dev/schemas/tasks.json",
            rename = "$schema"
        )]
        pub schema: String,

        /// Extends one or many task configuration files. Supports a relative
        /// file path or a secure URL.
        #[setting(extend, validate = validate::extends_from)]
        pub extends: Option<schematic::ExtendsFrom>,

        /// A mapping of group IDs to a list of file paths, globs, and
        /// environment variables, that can be referenced from tasks.
        #[setting(merge = merge_fxhashmap)]
        pub file_groups: FxHashMap<Id, Vec<InputPath>>,

        /// Task dependencies that'll automatically be injected into every
        /// task that inherits this configuration.
        #[setting(nested, merge = merge::append_vec, validate = validate_deps)]
        pub implicit_deps: Vec<TaskDependency>,

        /// Task inputs that'll automatically be injected into every
        /// task that inherits this configuration.
        #[setting(merge = merge::append_vec)]
        pub implicit_inputs: Vec<InputPath>,

        /// A mapping of tasks by ID to parameters required for running the task.
        #[setting(nested, merge = merge::merge_btreemap)]
        pub tasks: BTreeMap<Id, TaskConfig>,

        /// Default task options for all inherited tasks.
        #[setting(nested)]
        pub task_options: Option<TaskOptionsConfig>,
    }
);

cacheable!(
    #[derive(Clone, Debug, Default)]
    pub struct InheritedTasksResult {
        pub order: Vec<String>,
        pub config: InheritedTasksConfig,
        pub layers: IndexMap<String, PartialInheritedTasksConfig>,
        pub task_layers: FxHashMap<String, Vec<String>>,
    }
);

#[derive(Debug, Default)]
pub struct InheritedTasksEntry {
    pub input: PathBuf,
    pub config: PartialInheritedTasksConfig,
}

#[derive(Debug, Default)]
pub struct InheritedTasksManager {
    #[cfg(feature = "loader")]
    cache: Arc<RwLock<FxHashMap<String, InheritedTasksResult>>>,
    #[cfg(feature = "loader")]
    config_finder: crate::config_finder::ConfigFinder,

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
}

#[cfg(feature = "loader")]
impl InheritedTasksManager {
    pub fn add_config(
        &mut self,
        workspace_root: &Path,
        path: &Path,
        config: PartialInheritedTasksConfig,
    ) {
        let valid_names = self.config_finder.get_tasks_file_names();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        let name = if valid_names.iter().any(|n| n == name) {
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
        use crate::shapes::OneOrMany;
        use moon_common::color;
        use moon_common::path::standardize_separators;
        use schematic::{ConfigError, PartialConfig};

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
                                .push(InputPath::WorkspaceFile(source_path.clone()));

                            // Automatically set the toolchain
                            if task.toolchain.is_none() {
                                task.toolchain = Some(OneOrMany::Many(toolchains.to_owned()));
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
