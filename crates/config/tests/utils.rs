#![allow(dead_code, unused_imports)]

use indexmap::IndexMap;
use moon_common::Id;
pub use moon_config::test_utils::*;
use moon_config::*;
use moon_target::Target;
use proto_core::PluginLocator;
use rustc_hash::FxHashMap;
use schematic::{Config, ConfigError, ConfigLoader as BaseLoader};
use serde_json::Value;
use starbase_sandbox::{create_empty_sandbox, locate_fixture};
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

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

pub fn load_tasks_config_in_format(format: &str) {
    use starbase_sandbox::pretty_assertions::assert_eq;

    let config = test_config(locate_fixture(format), |path| {
        ConfigLoader::new(path.join(".moon"))
            .load_tasks_config_from_path(path.join(format!(".moon/tasks.{format}")))
    });

    assert_eq!(
        config,
        InheritedTasksConfig {
            file_groups: FxHashMap::from_iter([
                (
                    Id::raw("sources"),
                    vec![Input::Glob(stub_glob_input("src/**/*"))]
                ),
                (
                    Id::raw("tests"),
                    vec![
                        Input::Glob(stub_glob_input("*.test.ts")),
                        Input::Glob(stub_glob_input("*.test.tsx"))
                    ]
                ),
            ]),
            implicit_deps: vec![
                TaskDependency::Target(Target::parse("project:task-a").unwrap()),
                TaskDependency::Config(TaskDependencyConfig {
                    target: Target::parse("project:task-b").unwrap(),
                    optional: Some(true),
                    ..Default::default()
                }),
                TaskDependency::Target(Target::parse("project:task-c").unwrap()),
                TaskDependency::Config(TaskDependencyConfig {
                    args: TaskArgs::String("--foo --bar".into()),
                    env: FxHashMap::from_iter([("KEY".into(), "value".into())]),
                    target: Target::parse("project:task-d").unwrap(),
                    ..Default::default()
                }),
            ],
            implicit_inputs: vec![
                Input::EnvVar("ENV".into()),
                Input::EnvVarGlob("ENV_*".into()),
                Input::File(stub_file_input("file.txt")),
                Input::Glob(stub_glob_input("file.*")),
                Input::File(stub_file_input("/file.txt")),
                Input::Glob(stub_glob_input("/file.*")),
            ],
            task_options: Some(TaskOptionsConfig {
                affected_files: Some(TaskOptionAffectedFiles::Args),
                affected_pass_inputs: Some(true),
                allow_failure: Some(true),
                cache: Some(TaskOptionCache::Enabled(false)),
                cache_key: None,
                cache_lifetime: None,
                env_file: Some(TaskOptionEnvFile::File(FilePath(".env".into()))),
                infer_inputs: None,
                interactive: Some(false),
                internal: Some(true),
                merge: None,
                merge_args: Some(TaskMergeStrategy::Append),
                merge_deps: Some(TaskMergeStrategy::Prepend),
                merge_env: Some(TaskMergeStrategy::Replace),
                merge_inputs: Some(TaskMergeStrategy::Preserve),
                merge_outputs: None,
                merge_toolchains: None,
                mutex: Some("lock".into()),
                os: Some(OneOrMany::Many(vec![
                    TaskOperatingSystem::Linux,
                    TaskOperatingSystem::Macos
                ])),
                output_style: Some(TaskOutputStyle::Stream),
                persistent: Some(true),
                priority: None,
                retry_count: Some(3),
                run_deps_in_parallel: Some(false),
                run_in_ci: Some(TaskOptionRunInCI::Enabled(true)),
                run_from_workspace_root: Some(false),
                shell: Some(false),
                timeout: Some(60),
                unix_shell: Some(TaskUnixShell::Zsh),
                windows_shell: Some(TaskWindowsShell::Pwsh)
            }),
            tasks: BTreeMap::from_iter([
                (
                    Id::raw("build-linux"),
                    TaskConfig {
                        command: TaskArgs::String("cargo".into()),
                        args: TaskArgs::List(vec![
                            "--target".into(),
                            "x86_64-unknown-linux-gnu".into(),
                            "--verbose".into(),
                        ]),
                        options: TaskOptionsConfig {
                            os: Some(OneOrMany::One(TaskOperatingSystem::Linux)),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                ),
                (
                    Id::raw("build-macos"),
                    TaskConfig {
                        command: TaskArgs::String("cargo".into()),
                        args: TaskArgs::List(vec![
                            "--target".into(),
                            "x86_64-apple-darwin".into(),
                            "--verbose".into(),
                        ]),
                        options: TaskOptionsConfig {
                            os: Some(OneOrMany::One(TaskOperatingSystem::Macos)),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                ),
                (
                    Id::raw("build-windows"),
                    TaskConfig {
                        command: TaskArgs::String("cargo".into()),
                        args: TaskArgs::List(vec![
                            "--target".into(),
                            "i686-pc-windows-msvc".into(),
                            "--verbose".into(),
                        ]),
                        options: TaskOptionsConfig {
                            os: Some(OneOrMany::One(TaskOperatingSystem::Windows)),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                ),
                (
                    Id::raw("example"),
                    TaskConfig {
                        options: TaskOptionsConfig {
                            cache: Some(TaskOptionCache::Enabled(true)),
                            cache_lifetime: Some("1 hour".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                ),
                (
                    Id::raw("lint"),
                    TaskConfig {
                        inputs: Some(vec![
                            Input::Glob(stub_glob_input("**/*.graphql")),
                            Input::Glob(stub_glob_input("src/**/*")),
                        ]),
                        ..Default::default()
                    }
                ),
                (
                    Id::raw("test"),
                    TaskConfig {
                        inputs: Some(vec![
                            Input::Glob(stub_glob_input("src/**/*")),
                            Input::Glob(stub_glob_input("tests/**/*")),
                        ]),
                        ..Default::default()
                    }
                ),
            ]),
            ..Default::default()
        }
    );
}

pub fn load_template_config_in_format(format: &str) {
    use starbase_sandbox::pretty_assertions::assert_eq;

    let config = test_config(locate_fixture(format), |path| {
        ConfigLoader::new(path.join(".moon")).load_template_config(path)
    });

    assert_eq!(
        config,
        TemplateConfig {
            description: "Description".into(),
            destination: Some("./out".into()),
            id: Some(Id::raw("template-name")),
            title: "Title".into(),
            variables: FxHashMap::from_iter([
                (
                    "boolean".into(),
                    TemplateVariable::Boolean(TemplateVariableBoolSetting {
                        default: false,
                        internal: false,
                        order: None,
                        prompt: Some("Why?".into()),
                        required: Some(true)
                    })
                ),
                (
                    "enum".into(),
                    TemplateVariable::Enum(TemplateVariableEnumSetting {
                        default: TemplateVariableEnumDefault::default(),
                        internal: false,
                        multiple: Some(true),
                        order: Some(4),
                        prompt: None,
                        values: vec![
                            TemplateVariableEnumValue::String("a".into()),
                            TemplateVariableEnumValue::Object(TemplateVariableEnumValueConfig {
                                label: "B".into(),
                                value: "b".into()
                            }),
                            TemplateVariableEnumValue::String("c".into())
                        ]
                    })
                ),
                (
                    "number".into(),
                    TemplateVariable::Number(TemplateVariableNumberSetting {
                        default: 123,
                        internal: false,
                        order: Some(1),
                        prompt: Some("Why?".into()),
                        required: None
                    })
                ),
                (
                    "string".into(),
                    TemplateVariable::String(TemplateVariableStringSetting {
                        default: "abc".into(),
                        internal: true,
                        order: None,
                        prompt: Some("Why?".into()),
                        required: None
                    })
                ),
            ]),
            ..Default::default()
        }
    );
}

pub fn load_toolchains_config_in_format(format: &str) {
    use starbase_sandbox::pretty_assertions::assert_eq;

    let config = test_config(locate_fixture(format), |path| {
        let proto = proto_core::ProtoConfig::default();
        ConfigLoader::new(path.join(".moon")).load_toolchains_config(path, &proto)
    });

    assert_eq!(
        config.plugins.get("typescript").unwrap().config,
        BTreeMap::from_iter([
            ("createMissingConfig".into(), Value::Bool(false)),
            ("includeProjectReferenceSources".into(), Value::Bool(true)),
            ("includeSharedTypes".into(), Value::Bool(true)),
            (
                "projectConfigFileName".into(),
                Value::String("tsconfig.app.json".into())
            ),
            (
                "rootConfigFileName".into(),
                Value::String("tsconfig.root.json".into())
            ),
            (
                "rootOptionsConfigFileName".into(),
                Value::String("tsconfig.opts.json".into())
            ),
            ("routeOutDirToCache".into(), Value::Bool(true)),
            ("syncProjectReferences".into(), Value::Bool(false)),
            ("syncProjectReferencesToPaths".into(), Value::Bool(true)),
        ])
    );

    assert_eq!(
        config.plugins.get("node").unwrap(),
        &ToolchainPluginConfig {
            plugin: Some(PluginLocator::from_str("file://node.wasm").unwrap()),
            version: Some(UnresolvedVersionSpec::parse("20").unwrap()),
            ..Default::default()
        }
    );

    assert_eq!(config.proto.version.to_string(), "1.2.3");
}

pub fn load_extensions_config_in_format(format: &str) {
    use starbase_sandbox::pretty_assertions::assert_eq;

    let config = test_config(locate_fixture(format), |path| {
        ConfigLoader::new(path.join(".moon")).load_extensions_config(path)
    });

    assert_eq!(
        config.plugins.get("custom").unwrap(),
        &ExtensionPluginConfig {
            plugin: Some(PluginLocator::from_str("file://node.wasm").unwrap()),
            config: FxHashMap::from_iter([("key".into(), Value::String("value".into())),]),
            ..Default::default()
        }
    );
}

pub fn load_workspace_config_in_format(format: &str) {
    use starbase_sandbox::pretty_assertions::assert_eq;

    let config = test_config(locate_fixture(format), |path| {
        ConfigLoader::new(path.join(".moon")).load_workspace_config(path)
    });

    assert_eq!(
        config.codeowners,
        CodeownersConfig {
            global_paths: IndexMap::from_iter([("*".to_owned(), vec!["@admins".to_owned()])]),
            order_by: CodeownersOrderBy::ProjectId,
            required_approvals: Some(1),
            sync: true,
        }
    );
    assert_eq!(
        config.constraints,
        ConstraintsConfig {
            enforce_layer_relationships: false,
            tag_relationships: FxHashMap::from_iter([(
                Id::raw("a"),
                vec![Id::raw("b"), Id::raw("c")]
            )]),
        }
    );
    assert_eq!(
        config.docker,
        DockerConfig {
            prune: DockerPruneConfig {
                delete_vendor_directories: false,
                install_toolchain_dependencies: false
            },
            scaffold: DockerScaffoldConfig {
                configs_phase_globs: vec![GlobPath("*.js".into())],
                sources_phase_globs: vec![]
            },
            ..Default::default()
        }
    );
    assert_eq!(
        config.generator,
        GeneratorConfig {
            templates: vec![
                TemplateLocator::from_str("/shared-templates").unwrap(),
                TemplateLocator::from_str("./templates").unwrap()
            ]
        }
    );
    assert_eq!(
        config.hasher,
        HasherConfig {
            ignore_patterns: vec![GlobPath("*.map".into())],
            ignore_missing_patterns: vec![GlobPath(".env".into())],
            optimization: HasherOptimization::Performance,
            walk_strategy: HasherWalkStrategy::Vcs,
            warn_on_missing_inputs: true
        }
    );
    assert_eq!(
        config.notifier,
        NotifierConfig {
            terminal_notifications: None,
            webhook_url: Some("http://localhost".into()),
            webhook_acknowledge: false
        }
    );
    assert_eq!(
        config.projects,
        WorkspaceProjects::Both(WorkspaceProjectsConfig {
            globs: vec!["apps/*".into(), "packages/*".into()],
            glob_format: WorkspaceProjectGlobFormat::DirName,
            sources: FxHashMap::from_iter([(Id::raw("root"), ".".into())])
        })
    );
    assert_eq!(
        config.pipeline,
        PipelineConfig {
            auto_clean_cache: false,
            cache_lifetime: "1 day".into(),
            inherit_colors_for_piped_tasks: false,
            kill_process_threshold: 2000,
            log_running_command: true,
            ..Default::default()
        }
    );
    assert!(!config.telemetry);
    assert_eq!(
        config.vcs,
        VcsConfig {
            default_branch: "main".into(),
            hooks: FxHashMap::from_iter([(
                "pre-commit".into(),
                vec![
                    "moon check --all --affected".into(),
                    "moon run :pre-commit".into()
                ]
            )]),
            hook_format: VcsHookFormat::Native,
            client: VcsClient::Git,
            provider: VcsProvider::GitLab,
            remote_candidates: vec!["main".into(), "origin/main".into()],
            sync: true,
        }
    );
}
