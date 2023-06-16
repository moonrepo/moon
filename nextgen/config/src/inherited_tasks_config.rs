use crate::language_platform::{LanguageType, PlatformType};
use crate::project::{validate_deps, TaskConfig};
use crate::project_config::ProjectType;
use crate::shapes::InputPath;
use moon_common::cacheable;
use moon_common::{consts, Id};
use moon_target::Target;
use once_map::OnceMap;
use rustc_hash::FxHashMap;
use schematic::{merge, validate, Config, ConfigError, ConfigLoader, Layer, PartialConfig, Source};
use std::hash::Hash;
use std::{collections::BTreeMap, path::Path};

pub fn merge_fxhashmap<K, V, C>(
    mut prev: FxHashMap<K, V>,
    next: FxHashMap<K, V>,
    _context: &C,
) -> Result<Option<FxHashMap<K, V>>, ConfigError>
where
    K: Eq + Hash,
{
    for (key, value) in next {
        prev.insert(key, value);
    }

    Ok(Some(prev))
}

cacheable!(
    /// Docs: https://moonrepo.dev/docs/config/tasks
    #[derive(Clone, Config, Debug)]
    pub struct InheritedTasksConfig {
        #[setting(
            default = "https://moonrepo.dev/schemas/tasks.json",
            rename = "$schema"
        )]
        pub schema: String,

        #[setting(extend, validate = validate::extends_string)]
        pub extends: Option<String>,

        #[setting(merge = merge_fxhashmap)]
        pub file_groups: FxHashMap<Id, Vec<InputPath>>,

        #[setting(merge = merge::append_vec, validate = validate_deps)]
        pub implicit_deps: Vec<Target>,

        #[setting(merge = merge::append_vec)]
        pub implicit_inputs: Vec<InputPath>,

        #[setting(nested, merge = merge::merge_btreemap)]
        pub tasks: BTreeMap<Id, TaskConfig>,
    }
);

impl InheritedTasksConfig {
    pub fn load<F: AsRef<Path>>(path: F) -> Result<InheritedTasksConfig, ConfigError> {
        let result = ConfigLoader::<InheritedTasksConfig>::new()
            .file_optional(path.as_ref())?
            .load()?;

        Ok(result.config)
    }

    pub fn load_partial<T: AsRef<Path>, F: AsRef<Path>>(
        workspace_root: T,
        path: F,
    ) -> Result<PartialInheritedTasksConfig, ConfigError> {
        let workspace_root = workspace_root.as_ref();
        let path = path.as_ref();

        ConfigLoader::<InheritedTasksConfig>::new()
            .set_root(workspace_root)
            .file_optional(path)?
            .load_partial(&())
    }
}

fn is_js_platform(platform: &PlatformType) -> bool {
    matches!(platform, PlatformType::Deno | PlatformType::Node)
}

#[derive(Clone, Debug, Default)]
pub struct InheritedTasksResult {
    pub config: InheritedTasksConfig,
    pub layers: Vec<Layer<InheritedTasksConfig>>,
}

#[derive(Debug, Default)]
pub struct InheritedTasksManager {
    cache: OnceMap<String, InheritedTasksResult>,
    pub configs: FxHashMap<String, PartialInheritedTasksConfig>,
}

impl InheritedTasksManager {
    pub fn add_config(&mut self, path: &Path, config: PartialInheritedTasksConfig) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        let name = if name == consts::CONFIG_TASKS_FILENAME {
            "*"
        } else if let Some(stripped_name) = name.strip_suffix(".yml") {
            stripped_name
        } else {
            name
        };

        self.configs.insert(name.to_owned(), config);
    }

    pub fn get_lookup_order(
        &self,
        platform: &PlatformType,
        language: &LanguageType,
        project: &ProjectType,
        tags: &[Id],
    ) -> Vec<String> {
        let mut lookup = vec!["*".to_string()];

        if is_js_platform(platform) {
            lookup.push(format!("{platform}"));
        }

        lookup.push(format!("{language}"));

        if is_js_platform(platform) {
            lookup.push(format!("{platform}-{project}"));
        }

        lookup.push(format!("{language}-{project}"));

        for tag in tags {
            lookup.push(format!("tag-{tag}"));
        }

        lookup
    }

    pub fn get_inherited_config(
        &self,
        platform: &PlatformType,
        language: &LanguageType,
        project: &ProjectType,
        tags: &[Id],
    ) -> Result<InheritedTasksResult, ConfigError> {
        let lookup_order = self.get_lookup_order(platform, language, project, tags);
        let lookup_key = lookup_order.join(":");

        // Cache the result as this lookup may be the same for a large number of projects,
        // and since this clones constantly, we can avoid a lot of allocations and overhead.
        self.cache.try_insert_cloned(lookup_key, |_| {
            let mut partial_config = PartialInheritedTasksConfig::default();
            let mut layers = vec![];
            let context = ();

            for lookup in lookup_order {
                if let Some(managed_config) = self.configs.get(&lookup) {
                    let mut managed_config = managed_config.clone();

                    let source_path = if lookup == "*" {
                        format!(
                            "{}/{}",
                            consts::CONFIG_DIRNAME,
                            consts::CONFIG_TASKS_FILENAME
                        )
                    } else {
                        format!("{}/tasks/{lookup}.yml", consts::CONFIG_DIRNAME)
                    };

                    if let Some(tasks) = &mut managed_config.tasks {
                        for task in tasks.values_mut() {
                            // Automatically set this lookup as an input
                            task.global_inputs
                                .get_or_insert(vec![])
                                .push(InputPath::WorkspaceFile(source_path.clone()));

                            // Automatically set the platform
                            if task.platform.unwrap_or_default().is_unknown() {
                                task.platform = Some(platform.to_owned());
                            }
                        }
                    }

                    layers.push(Layer {
                        partial: managed_config.clone(),
                        source: Source::file(source_path, true)?,
                    });

                    partial_config.merge(&context, managed_config)?;
                }
            }

            let config = partial_config.finalize(&context)?;

            config
                .validate(&context)
                .map_err(|error| ConfigError::Validator {
                    config: format!(
                        "inherited tasks {}",
                        if is_js_platform(platform) {
                            format!("({}, {}, {})", platform, language, project)
                        } else {
                            format!("({}, {})", language, project)
                        }
                    ),
                    error,
                })?;

            Ok(InheritedTasksResult {
                config: InheritedTasksConfig::from_partial(config),
                layers,
            })
        })
    }
}
