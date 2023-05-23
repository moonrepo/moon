use crate::language_platform::{LanguageType, PlatformType};
use crate::project::TaskConfig;
use crate::project_config::ProjectType;
use crate::relative_path::RelativePath;
use crate::FilePath;
use moon_common::{consts, Id};
use moon_target::Target;
use rustc_hash::FxHashMap;
use schematic::{color, merge, validate, Config, ConfigError, ConfigLoader, PartialConfig};
use std::hash::Hash;
use std::{collections::BTreeMap, path::Path};

pub fn merge_fxhashmap<K, V, C>(
    mut prev: FxHashMap<K, V>,
    next: FxHashMap<K, V>,
    _: &C,
) -> Result<Option<FxHashMap<K, V>>, ConfigError>
where
    K: Eq + Hash,
{
    for (key, value) in next {
        prev.insert(key, value);
    }

    Ok(Some(prev))
}

/// Docs: https://moonrepo.dev/docs/config/tasks
#[derive(Debug, Clone, Config)]
pub struct InheritedTasksConfig {
    #[setting(
        default = "https://moonrepo.dev/schemas/tasks.json",
        rename = "$schema"
    )]
    pub schema: String,

    #[setting(extend, validate = validate::extends_string)]
    pub extends: Option<String>,

    #[setting(merge = merge_fxhashmap)]
    pub file_groups: FxHashMap<Id, Vec<RelativePath>>,

    #[setting(merge = merge::append_vec)]
    pub implicit_deps: Vec<Target>,

    #[setting(merge = merge::append_vec)]
    pub implicit_inputs: Vec<RelativePath>,

    #[setting(nested, merge = merge::merge_btreemap)]
    pub tasks: BTreeMap<Id, TaskConfig>,
}

impl InheritedTasksConfig {
    pub fn load<T: AsRef<Path>, F: AsRef<Path>>(
        workspace_root: T,
        path: F,
    ) -> Result<InheritedTasksConfig, ConfigError> {
        let workspace_root = workspace_root.as_ref();
        let path = path.as_ref();

        let result = ConfigLoader::<InheritedTasksConfig>::yaml()
            .label(color::path(path))
            .file(workspace_root.join(path))?
            .load()?;

        Ok(result.config)
    }
}

#[derive(Debug, Default)]
pub struct InheritedTasksManager {
    pub configs: FxHashMap<String, PartialInheritedTasksConfig>,
}

impl InheritedTasksManager {
    pub fn add_config(&mut self, path: &Path, config: PartialInheritedTasksConfig) {
        let name = path.file_name().unwrap_or_default().to_str().unwrap();

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

        // JS/TS is special in that it runs on multiple platforms
        let is_js_platform = matches!(platform, PlatformType::Deno | PlatformType::Node);

        if is_js_platform {
            lookup.push(format!("{platform}"));
        }

        lookup.push(format!("{language}"));

        if is_js_platform {
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
    ) -> Result<InheritedTasksConfig, ConfigError> {
        let mut config = PartialInheritedTasksConfig::default();

        for lookup in self.get_lookup_order(platform, language, project, tags) {
            if let Some(managed_config) = self.configs.get(&lookup) {
                let mut managed_config = managed_config.clone();

                if lookup != "*" {
                    if let Some(tasks) = &mut managed_config.tasks {
                        for task in tasks.values_mut() {
                            // Automatically set this lookup as an input
                            let global_lookup = RelativePath::WorkspaceFile(FilePath(format!(
                                ".moon/tasks/{lookup}.yml"
                            )));

                            if let Some(global_inputs) = &mut task.global_inputs {
                                global_inputs.push(global_lookup);
                            } else {
                                task.global_inputs = Some(vec![global_lookup]);
                            }

                            // Automatically set the platform
                            if task.platform.clone().unwrap_or_default().is_unknown() {
                                task.platform = Some(platform.to_owned());
                            }
                        }
                    }
                }

                config.merge(&(), managed_config)?;
            }
        }

        InheritedTasksConfig::from_partial(&(), config, false)
    }
}
