#![allow(dead_code, unused_imports)]

use moon_common::Id;
pub use moon_config::test_utils::*;
use moon_config::*;
use moon_target::Target;
use rustc_hash::FxHashMap;
use schematic::{Config, ConfigError, ConfigLoader as BaseLoader};
use starbase_sandbox::{create_empty_sandbox, locate_fixture};
use std::collections::BTreeMap;
use std::path::Path;

pub fn unwrap_config_result<T>(result: miette::Result<T>) -> T {
    match result {
        Ok(config) => config,
        Err(error) => {
            panic!(
                "{}",
                error.downcast::<ConfigError>().unwrap().to_full_string()
            )
        }
    }
}

pub fn test_config<P, T, F>(path: P, callback: F) -> T
where
    P: AsRef<Path>,
    T: Config,
    F: FnOnce(&Path) -> miette::Result<T>,
{
    unwrap_config_result(callback(path.as_ref()))
}

pub fn test_load_config<T, F>(file: &str, code: &str, callback: F) -> T
where
    T: Config,
    F: FnOnce(&Path) -> miette::Result<T>,
{
    let sandbox = create_empty_sandbox();

    sandbox.create_file(file, code);

    unwrap_config_result(callback(sandbox.path()))
}

pub fn test_parse_config<T, F>(code: &str, callback: F) -> T
where
    T: Config,
    F: FnOnce(&str) -> miette::Result<T>,
{
    unwrap_config_result(callback(code))
}

pub fn load_project_config_in_format(format: &str) {
    use starbase_sandbox::pretty_assertions::assert_eq;

    let config = test_config(locate_fixture(format), |path| {
        ConfigLoader::new(path.join(".moon")).load_project_config(path)
    });

    assert_eq!(
        config,
        ProjectConfig {
            depends_on: vec![
                ProjectDependsOn::String(Id::raw("a")),
                ProjectDependsOn::Object(ProjectDependencyConfig {
                    id: Id::raw("b"),
                    scope: DependencyScope::Build,
                    source: DependencySource::Implicit,
                    via: None
                })
            ],
            docker: ProjectDockerConfig {
                file: DockerFileConfig {
                    build_task: Some(Id::raw("build")),
                    image: Some("node:latest".into()),
                    start_task: Some(Id::raw("start")),
                    ..Default::default()
                },
                scaffold: DockerScaffoldConfig {
                    configs_phase_globs: vec![],
                    sources_phase_globs: vec![GlobPath("*.js".into())]
                }
            },
            env: FxHashMap::from_iter([("KEY".into(), "value".into())]),
            file_groups: FxHashMap::from_iter([
                (
                    Id::raw("sources"),
                    vec![Input::Glob(stub_glob_input("src/**/*"))]
                ),
                (
                    Id::raw("tests"),
                    vec![Input::Glob(stub_glob_input("/**/*.test.*"))]
                )
            ]),
            id: Some(Id::raw("custom-id")),
            language: LanguageType::Rust,
            owners: OwnersConfig {
                custom_groups: FxHashMap::default(),
                default_owner: Some("owner".into()),
                optional: true,
                paths: OwnersPaths::List(vec![
                    GlobPath::parse("dir/").unwrap(),
                    GlobPath::parse("file.txt").unwrap()
                ]),
                required_approvals: Some(5)
            },
            project: Some(ProjectMetadataConfig {
                title: Some("Name".into()),
                description: Some("Does something".into()),
                owner: Some("team".into()),
                maintainers: vec![],
                channel: Some("#team".into()),
                metadata: FxHashMap::from_iter([
                    ("bool".into(), serde_json::Value::Bool(true)),
                    ("string".into(), serde_json::Value::String("abc".into()))
                ]),
            }),
            stack: StackType::Frontend,
            tags: vec![Id::raw("a"), Id::raw("b"), Id::raw("c")],
            tasks: BTreeMap::default(),
            toolchains: ProjectToolchainsConfig {
                plugins: FxHashMap::from_iter([
                    (
                        Id::raw("deno"),
                        ProjectToolchainEntry::Config(ToolchainPluginConfig {
                            version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        Id::raw("typescript"),
                        ProjectToolchainEntry::Config(ToolchainPluginConfig {
                            config: BTreeMap::from_iter([(
                                "includeSharedTypes".into(),
                                serde_json::Value::Bool(true)
                            )]),
                            ..Default::default()
                        })
                    )
                ]),
                ..Default::default()
            },
            layer: LayerType::Library,
            workspace: ProjectWorkspaceConfig {
                inherited_tasks: ProjectWorkspaceInheritedTasksConfig {
                    exclude: vec![Id::raw("build")],
                    include: Some(vec![Id::raw("test")]),
                    rename: FxHashMap::from_iter([(Id::raw("old"), Id::raw("new"))])
                }
            },
            ..Default::default()
        }
    );
}

pub fn load_task_config_in_format(format: &str) {
    use starbase_sandbox::pretty_assertions::assert_eq;

    let config = test_config(locate_fixture(format), |root| {
        let mut loader = BaseLoader::<TaskConfig>::new();

        ConfigLoader::default()
            .prepare_loader(&mut loader, vec![root.join(format!("task.{format}"))])
            .unwrap();

        Ok(loader.load()?.config)
    });

    assert_eq!(
        config,
        TaskConfig {
            description: Some("I do something".into()),
            command: TaskArgs::String("cmd --arg".into()),
            args: TaskArgs::List(vec!["-c".into(), "-b".into(), "arg".into()]),
            deps: Some(vec![
                TaskDependency::Target(Target::parse("proj:task").unwrap()),
                TaskDependency::Config(TaskDependencyConfig {
                    args: TaskArgs::None,
                    env: FxHashMap::default(),
                    target: Target::parse("^:build").unwrap(),
                    optional: Some(true)
                }),
                TaskDependency::Config(TaskDependencyConfig {
                    args: TaskArgs::String("--minify".into()),
                    env: FxHashMap::from_iter([("DEBUG".into(), "1".into())]),
                    target: Target::parse("~:build").unwrap(),
                    optional: None
                }),
            ]),
            env: Some(FxHashMap::from_iter([("ENV".into(), "development".into())])),
            inputs: Some(vec![
                Input::EnvVar("ENV".into()),
                Input::EnvVarGlob("ENV_*".into()),
                Input::File(stub_file_input("file.txt")),
                Input::Glob(stub_glob_input("file.*")),
                Input::File(stub_file_input("/file.txt")),
                Input::Glob(stub_glob_input("/file.*")),
                Input::TokenFunc("@dirs(name)".into())
            ]),
            outputs: Some(vec![
                Output::TokenVar("$workspaceRoot".into()),
                Output::File(stub_file_output("file.txt")),
                Output::Glob(stub_glob_output("file.*")),
                Output::File(stub_file_output("/file.txt")),
                Output::Glob(stub_glob_output("/file.*")),
            ]),
            options: TaskOptionsConfig {
                cache: Some(TaskOptionCache::Enabled(false)),
                retry_count: Some(3),
                ..Default::default()
            },
            preset: Some(TaskPreset::Server),
            type_of: Some(TaskType::Build),
            ..Default::default()
        }
    );
}
