use moon_config::*;
use moon_target::Target;
use schematic::ConfigLoader;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

// Full config structs are serialized and deserialized directly (not through
// their partials), like in the workspace graph cache, so every field that is
// skipped when serializing must be defaulted when deserializing
fn round_trip<T: Debug + PartialEq + Serialize + DeserializeOwned>(config: T) {
    let json = serde_json::to_value(&config).unwrap();
    let parsed: T = serde_json::from_value(json).unwrap();

    assert_eq!(parsed, config);
}

mod full_struct_serde {
    use super::*;

    #[test]
    fn round_trips_default_root_configs() {
        round_trip(ExtensionsConfig::default());
        round_trip(InheritedTasksConfig::default());
        round_trip(ProjectConfig::default());
        round_trip(TemplateConfig::default());
        round_trip(TemplateFrontmatterConfig::default());
        round_trip(ToolchainsConfig::default());
    }

    #[test]
    fn round_trips_default_workspace_config() {
        // Template locators normalize `./templates` to `templates` when parsed,
        // so start from a config that has already been read back once
        let json = serde_json::to_value(WorkspaceConfig::default()).unwrap();

        round_trip(serde_json::from_value::<WorkspaceConfig>(json).unwrap());
    }

    #[test]
    fn round_trips_task_with_object_deps() {
        round_trip(TaskConfig {
            deps: Some(vec![
                TaskDependency::Object(TaskDependencyConfig::new(
                    Target::parse("project:build").unwrap(),
                )),
                TaskDependency::Object(TaskDependencyConfig {
                    type_of: TaskDependencyType::Cleanup,
                    ..TaskDependencyConfig::new(Target::parse("project:stop").unwrap())
                }),
            ]),
            ..Default::default()
        });
    }

    // Inputs, outputs, and checks are deserialized through their partial shapes,
    // in both config files and the workspace graph cache
    #[test]
    fn round_trips_task_with_input_output_and_check_shapes() {
        let config = ConfigLoader::<TaskConfig>::new()
            .code(
                r#"
inputs:
  - '$ENV_VAR'
  - 'file.txt'
  - { file: 'other.txt', optional: true }
  - { glob: 'src/**/*', cache: false }
  - { glob: 'tests/**/*' }
  - { group: 'sources', format: 'files' }
  - { project: 'app', group: 'sources', filter: ['src/**/*'] }
outputs:
  - 'dist/out.js'
  - { file: 'dist/maybe.js', optional: true }
  - { glob: 'dist/**/*.map' }
checks:
  - 'test -f file.txt'
  - { check: 'condition', script: 'test -n "$CI"' }
  - { check: 'fingerprint', script: 'git rev-parse HEAD', hash: 'stdout' }
"#,
                "task.yml",
            )
            .unwrap()
            .load()
            .unwrap()
            .config;

        assert_eq!(config.inputs.as_ref().unwrap().len(), 7);
        assert_eq!(config.outputs.as_ref().unwrap().len(), 3);
        assert_eq!(config.checks.as_ref().unwrap().len(), 3);

        round_trip(config);
    }
}
