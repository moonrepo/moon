use crate::language_platform::{LanguageType, PlatformType};
use crate::project::TaskConfig;
use crate::project_config::ProjectType;
use crate::relative_path::RelativePath;
use crate::FilePath;
use moon_common::Id;
use moon_target::Target;
use rustc_hash::FxHashMap;
use schematic::{merge, validate, Config, ConfigError, ConfigLoader};
use std::{collections::BTreeMap, path::Path};

/// Docs: https://moonrepo.dev/docs/config/tasks
#[derive(Debug, Default, Clone, Config)]
pub struct InheritedTasksConfig {
    #[setting(
        default = "https://moonrepo.dev/schemas/tasks.json",
        rename = "$schema"
    )]
    pub schema: String,

    #[setting(extend, validate = validate::extends_string)]
    pub extends: Option<String>,

    // #[setting(merge = merge::merge_hashmap)]
    pub file_groups: FxHashMap<Id, Vec<RelativePath>>,

    #[setting(merge = merge::append_vec)]
    pub implicit_deps: Vec<Target>,

    #[setting(merge = merge::append_vec)]
    pub implicit_inputs: Vec<RelativePath>,

    #[setting(nested, merge = merge::merge_btreemap)]
    pub tasks: BTreeMap<Id, TaskConfig>,
}

impl InheritedTasksConfig {
    // Figment does not merge maps/vec but replaces entirely,
    // so we need to manually handle this here!
    pub fn merge(&mut self, config: InheritedTasksConfig) {
        if !config.file_groups.is_empty() {
            self.file_groups.extend(config.file_groups);
        }

        if !config.implicit_deps.is_empty() {
            self.implicit_deps.extend(config.implicit_deps);
        }

        if !config.implicit_inputs.is_empty() {
            self.implicit_inputs.extend(config.implicit_inputs);
        }

        if !config.tasks.is_empty() {
            self.tasks.extend(config.tasks);
        }
    }

    pub fn load<T: AsRef<Path>>(path: T) -> Result<InheritedTasksConfig, ConfigError> {
        let result = ConfigLoader::<InheritedTasksConfig>::yaml()
            .file(path.as_ref())?
            .load()?;

        Ok(result.config)
    }
}

#[derive(Debug, Default)]
pub struct InheritedTasksManager {
    pub configs: FxHashMap<String, InheritedTasksConfig>,
}

impl InheritedTasksManager {
    pub fn add_config(&mut self, path: &Path, config: InheritedTasksConfig) {
        let name = path.file_name().unwrap_or_default().to_str().unwrap();

        let name = if name == "tasks.yml" {
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
    ) -> InheritedTasksConfig {
        let mut config = InheritedTasksConfig::default();

        for lookup in self.get_lookup_order(platform, language, project, tags) {
            if let Some(managed_config) = self.configs.get(&lookup) {
                let mut managed_config = managed_config.clone();

                for task in managed_config.tasks.values_mut() {
                    if lookup != "*" {
                        // Automatically set this lookup as an input
                        task.global_inputs
                            .push(RelativePath::WorkspaceFile(FilePath(format!(
                                ".moon/tasks/{lookup}.yml"
                            ))));

                        // Automatically set the platform
                        if task.platform.is_unknown() {
                            task.platform = platform.to_owned();
                        }
                    }
                }

                config.merge(managed_config);
            }
        }

        config
    }
}
