use crate::{InheritedTasksConfig, PlatformType, ProjectLanguage, ProjectType};
use moon_utils::{fs, string_vec};
use rustc_hash::FxHashMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct InheritedTasksManager {
    pub configs: FxHashMap<String, InheritedTasksConfig>,
}

impl InheritedTasksManager {
    pub fn add_config(&mut self, path: &Path, config: InheritedTasksConfig) {
        let name = fs::file_name(path);
        let name = if name == "tasks.yml" {
            "*"
        } else if name.ends_with(".yml") {
            name.strip_suffix(".yml").unwrap()
        } else {
            name.as_ref()
        };

        self.configs.insert(name.to_owned(), config);
    }

    pub fn get_lookup_order(
        &self,
        platform: PlatformType,
        language: ProjectLanguage,
        type_of: ProjectType,
    ) -> Vec<String> {
        let mut lookup = string_vec!["*"];

        // JS/TS is special in that it runs on multiple platforms
        let is_js_platform = matches!(platform, PlatformType::Node);

        if is_js_platform {
            lookup.push(format!("{platform}"));
        }

        lookup.push(format!("{language}"));

        if is_js_platform {
            lookup.push(format!("{platform}-{type_of}"));
        }

        lookup.push(format!("{language}-{type_of}"));

        lookup
    }

    pub fn get_inherited_config(
        &self,
        platform: PlatformType,
        language: ProjectLanguage,
        type_of: ProjectType,
    ) -> InheritedTasksConfig {
        let mut config = InheritedTasksConfig::default();

        for lookup in self.get_lookup_order(platform, language, type_of) {
            if let Some(managed_config) = self.configs.get(&lookup) {
                let mut managed_config = managed_config.clone();

                if lookup != "*" {
                    for task in managed_config.tasks.values_mut() {
                        // Automatically set the platform
                        if task.platform.is_unknown() {
                            task.platform = platform;
                        }

                        // Automatically set this lookup as an input
                        let input = format!("/.moon/tasks/{lookup}.yml");

                        if let Some(inputs) = &mut task.inputs {
                            inputs.push(input);
                        } else {
                            task.inputs = Some(vec![input]);
                        }
                    }
                }

                config.merge(managed_config);
            }
        }

        // Always break cache if a core configuration changes
        config.implicit_inputs.push("/.moon/*.yml".into());

        config
    }
}
