use crate::language_platform::{LanguageType, PlatformType};
use crate::project::{validate_deps, TaskConfig, TaskDependency, TaskOptionsConfig};
use crate::project_config::{ProjectType, StackType};
use crate::shapes::InputPath;
use moon_common::{cacheable, Id};
use rustc_hash::{FxHashMap, FxHasher};
use schematic::schema::{IndexMap, IndexSet};
use schematic::{merge, validate, Config, MergeResult};
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

cacheable!(
    /// Configures tasks and task related settings that'll be inherited by all
    /// matching projects.
    /// Docs: https://moonrepo.dev/docs/config/tasks
    #[derive(Clone, Config, Debug)]
    pub struct InheritedTasksConfig {
        #[setting(
            default = "https://moonrepo.dev/schemas/tasks.json",
            rename = "$schema"
        )]
        pub schema: String,

        /// Extends another tasks configuration file. Supports a relative
        /// file path or a secure URL.
        #[setting(extend, validate = validate::extends_string)]
        pub extends: Option<String>,

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

#[cfg(feature = "loader")]
impl InheritedTasksConfig {
    /// Only used in testing!
    pub fn load<F: AsRef<Path>>(path: F) -> miette::Result<InheritedTasksConfig> {
        use schematic::ConfigLoader;

        let result = ConfigLoader::<InheritedTasksConfig>::new()
            .file_optional(path.as_ref())?
            .load()?;

        Ok(result.config)
    }

    pub fn load_partial<T: AsRef<Path>, F: AsRef<Path>>(
        workspace_root: T,
        path: F,
    ) -> miette::Result<PartialInheritedTasksConfig> {
        use crate::config_cache::ConfigCache;
        use crate::validate::check_yml_extension;
        use moon_common::color;
        use schematic::ConfigLoader;

        let root = workspace_root.as_ref();

        Ok(ConfigLoader::<InheritedTasksConfig>::new()
            .set_cacher(ConfigCache::new(root))
            .set_help(color::muted_light("https://moonrepo.dev/docs/config/tasks"))
            .set_root(root)
            .file_optional(check_yml_extension(path.as_ref()))?
            .load_partial(&())?)
    }
}

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

    pub configs: FxHashMap<String, InheritedTasksEntry>,
}

impl InheritedTasksManager {
    pub fn get_lookup_order(
        &self,
        platform: &PlatformType,
        language: &LanguageType,
        stack: &StackType,
        project: &ProjectType,
        tags: &[Id],
    ) -> Vec<String> {
        let mut lookup: IndexSet<String, BuildHasherDefault<FxHasher>> = IndexSet::from_iter([
            "*".to_string(),
            format!("{platform}"), // node
            format!("{language}"), // javascript
            format!("{stack}"),    // frontend
            //
            format!("{platform}-{stack}"), // node-frontend
            format!("{language}-{stack}"), // javascript-frontend
            //
            format!("{stack}-{project}"),    // frontend-library
            format!("{platform}-{project}"), // node-library
            format!("{language}-{project}"), // javascript-library
            //
            format!("{platform}-{stack}-{project}"), // node-frontend-library
            format!("{language}-{stack}-{project}"), // javascript-frontend-library
        ]);

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
    pub fn load<T: AsRef<Path>, D: AsRef<Path>>(
        workspace_root: T,
        moon_dir: D,
    ) -> miette::Result<InheritedTasksManager> {
        use moon_common::consts::*;
        use moon_common::supports_pkl_configs;

        let mut manager = InheritedTasksManager::default();
        let workspace_root = workspace_root.as_ref();
        let moon_dir = moon_dir.as_ref();

        // tasks.*
        let yml_file = moon_dir.join(CONFIG_TASKS_FILENAME_YML);

        if yml_file.exists() {
            manager.add_config(
                workspace_root,
                &yml_file,
                InheritedTasksConfig::load_partial(workspace_root, &yml_file)?,
            );
        }

        if supports_pkl_configs() {
            let pkl_file = moon_dir.join(CONFIG_TASKS_FILENAME_PKL);

            if pkl_file.exists() {
                manager.add_config(
                    workspace_root,
                    &pkl_file,
                    InheritedTasksConfig::load_partial(workspace_root, &pkl_file)?,
                );
            }
        }

        // tasks/**/*.*
        let tasks_dir = moon_dir.join("tasks");

        if tasks_dir.exists() {
            load_dir(&mut manager, workspace_root, &tasks_dir)?;
        }

        Ok(manager)
    }

    pub fn load_from<T: AsRef<Path>>(workspace_root: T) -> miette::Result<InheritedTasksManager> {
        use moon_common::consts;

        let workspace_root = workspace_root.as_ref();

        Self::load(workspace_root, workspace_root.join(consts::CONFIG_DIRNAME))
    }

    pub fn add_config(
        &mut self,
        workspace_root: &Path,
        path: &Path,
        config: PartialInheritedTasksConfig,
    ) {
        use moon_common::consts::*;

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        let name = if name == CONFIG_TASKS_FILENAME_YML || name == CONFIG_TASKS_FILENAME_PKL {
            "*"
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
        platform: &PlatformType,
        language: &LanguageType,
        stack: &StackType,
        project: &ProjectType,
        tags: &[Id],
    ) -> miette::Result<InheritedTasksResult> {
        use moon_common::color;
        use moon_common::path::standardize_separators;
        use schematic::{ConfigError, PartialConfig};

        let lookup_order = self.get_lookup_order(platform, language, stack, project, tags);
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

                // Only modify tasks for `tasks/*.*` files instead of `tasks.*`,
                // as the latter will be globbed alongside toolchain/workspace configs.
                // We also don't know what platform each of the tasks should be yet.
                if let Some(tasks) = &mut managed_config.tasks {
                    for (task_id, task) in tasks.iter_mut() {
                        if lookup != "*" {
                            // Automatically set this source as an input
                            task.global_inputs
                                .get_or_insert(vec![])
                                .push(InputPath::WorkspaceFile(source_path.clone()));

                            // Automatically set the platform
                            if task.platform.unwrap_or_default().is_unknown() {
                                task.platform = Some(platform.to_owned());
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
            .map_err(|error| ConfigError::Validator {
                config: format!(
                    "inherited tasks {}",
                    if platform.is_javascript() {
                        format!("({}, {}, {}, {})", platform, language, stack, project)
                    } else {
                        format!("({}, {}, {})", language, stack, project)
                    }
                ),
                error: Box::new(error),
                help: Some(color::muted_light("https://moonrepo.dev/docs/config/tasks")),
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

#[cfg(feature = "loader")]
fn load_dir(
    manager: &mut InheritedTasksManager,
    workspace_root: &Path,
    dir: &Path,
) -> miette::Result<()> {
    use moon_common::supports_pkl_configs;
    use schematic::ConfigError;
    use std::fs;

    let use_pkl = supports_pkl_configs();

    for entry in fs::read_dir(dir)
        .map_err(|error| ConfigError::ReadFileFailed {
            path: dir.to_path_buf(),
            error: Box::new(error),
        })?
        .flatten()
    {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| ConfigError::ReadFileFailed {
                path: path.to_path_buf(),
                error: Box::new(error),
            })?;

        if file_type.is_file() {
            // Non-yaml/pkl files may be located in these folders,
            // so avoid failing when trying to parse it as a config
            if path
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml" || use_pkl && ext == "pkl")
            {
                manager.add_config(
                    workspace_root,
                    &path,
                    InheritedTasksConfig::load_partial(workspace_root, &path)?,
                );
            }
        } else if file_type.is_dir() {
            load_dir(manager, workspace_root, &path)?;
        }
    }

    Ok(())
}
