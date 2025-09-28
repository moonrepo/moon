#![allow(deprecated)] // For local

mod utils;

use moon_common::Id;
use moon_config::{
    FileGroupInput, FileGroupInputFormat, FilePath, Input, OneOrMany, Output, PlatformType,
    ProjectInput, TaskArgs, TaskConfig, TaskDependency, TaskDependencyConfig, TaskMergeStrategy,
    TaskOptionCache, TaskOutputStyle, TaskType,
};
use moon_target::Target;
use rustc_hash::FxHashMap;
use schematic::{ConfigLoader as BaseLoader, Format, RegexSetting};
use std::path::Path;
use utils::*;

fn load_config_from_code(code: &str) -> miette::Result<TaskConfig> {
    Ok(BaseLoader::<TaskConfig>::new()
        .code(code, Format::Yaml)?
        .load()?
        .config)
}

fn load_config_from_file(path: &Path) -> miette::Result<TaskConfig> {
    Ok(BaseLoader::<TaskConfig>::new().file(path)?.load()?.config)
}

mod task_config {
    use super::*;

    #[test]
    #[should_panic(
        expected = "unknown field `unknown`, expected one of `extends`, `description`, `command`, `args`, `deps`, `env`, `inputs`, `local`, `outputs`, `options`, `platform`, `preset`, `script`, `toolchain`, `toolchains`, `type`"
    )]
    fn error_unknown_field() {
        test_parse_config("unknown: 123", load_config_from_code);
    }

    #[test]
    fn loads_defaults() {
        let config = test_parse_config("{}", load_config_from_code);

        assert_eq!(config.command, TaskArgs::None);
        assert_eq!(config.args, TaskArgs::None);
        assert_eq!(config.type_of, None);
    }

    #[test]
    fn can_extend() {
        let config = test_parse_config("extends: id", load_config_from_code);

        assert_eq!(config.extends, Some(Id::raw("id")));
    }

    mod command {
        use super::*;

        #[test]
        #[should_panic(expected = "expected a string or a list of strings")]
        fn errors_on_invalid_type() {
            test_parse_config("command: 123", load_config_from_code);
        }

        #[test]
        #[should_panic(expected = "a command is required; use \"noop\" otherwise")]
        fn errors_for_empty_string() {
            test_parse_config("command: ''", load_config_from_code);
        }

        #[test]
        #[should_panic(expected = "a command is required; use \"noop\" otherwise")]
        fn errors_for_empty_list() {
            test_parse_config("command: []", load_config_from_code);
        }

        #[test]
        #[should_panic(expected = "a command is required; use \"noop\" otherwise")]
        fn errors_for_empty_list_arg() {
            test_parse_config("command: ['']", load_config_from_code);
        }

        #[test]
        fn parses_string() {
            let config = test_parse_config("command: bin", load_config_from_code);

            assert_eq!(config.command, TaskArgs::String("bin".into()));
        }

        #[test]
        fn parses_list() {
            let config = test_parse_config("command: [bin]", load_config_from_code);

            assert_eq!(config.command, TaskArgs::List(vec!["bin".into()]));
        }
    }

    mod args {
        use super::*;

        #[test]
        fn parses_string() {
            let config = test_parse_config("args: bin", load_config_from_code);

            assert_eq!(config.args, TaskArgs::String("bin".into()));
        }

        #[test]
        fn parses_list() {
            let config = test_parse_config("args: [bin]", load_config_from_code);

            assert_eq!(config.args, TaskArgs::List(vec!["bin".into()]));
        }

        #[test]
        fn supports_variants() {
            let config = test_parse_config(
                r"
args:
  - arg
  - -o
  - '@token(0)'
  - --opt
  - value
  - 'quoted arg'
",
                load_config_from_code,
            );

            assert_eq!(
                config.args,
                TaskArgs::List(vec![
                    "arg".into(),
                    "-o".into(),
                    "@token(0)".into(),
                    "--opt".into(),
                    "value".into(),
                    "quoted arg".into(),
                ])
            );
        }
    }

    mod deps {
        use super::*;

        #[test]
        fn supports_targets() {
            let config = test_parse_config(
                r"
deps:
  - task
  - project:task
  - ^:task
  - ~:task
",
                load_config_from_code,
            );

            assert_eq!(
                config.deps,
                Some(vec![
                    TaskDependency::Target(Target::parse("task").unwrap()),
                    TaskDependency::Target(Target::parse("project:task").unwrap()),
                    TaskDependency::Target(Target::parse("^:task").unwrap()),
                    TaskDependency::Target(Target::parse("~:task").unwrap()),
                ])
            );
        }

        #[test]
        fn supports_configs() {
            let config = test_parse_config(
                r"
deps:
  - target: task
  - args: a b c
    target: project:task
  - env:
      FOO: abc
    target: ^:task
  - args:
      - a
      - b
      - c
    env:
      FOO: abc
    target: ~:task
",
                load_config_from_code,
            );

            assert_eq!(
                config.deps,
                Some(vec![
                    TaskDependency::Config(TaskDependencyConfig::new(
                        Target::parse("task").unwrap()
                    )),
                    TaskDependency::Config(TaskDependencyConfig {
                        args: TaskArgs::String("a b c".into()),
                        target: Target::parse("project:task").unwrap(),
                        ..TaskDependencyConfig::default()
                    }),
                    TaskDependency::Config(TaskDependencyConfig {
                        env: FxHashMap::from_iter([("FOO".into(), "abc".into())]),
                        target: Target::parse("^:task").unwrap(),
                        ..TaskDependencyConfig::default()
                    }),
                    TaskDependency::Config(TaskDependencyConfig {
                        args: TaskArgs::List(vec!["a".into(), "b".into(), "c".into()]),
                        env: FxHashMap::from_iter([("FOO".into(), "abc".into())]),
                        target: Target::parse("~:task").unwrap(),
                        optional: None,
                    }),
                ])
            );
        }

        #[test]
        #[should_panic(expected = "expected a valid target or dependency config object")]
        fn errors_on_invalid_format() {
            test_parse_config("deps: ['bad target']", load_config_from_code);
        }

        #[test]
        #[should_panic(expected = "target scope not supported as a task dependency")]
        fn errors_on_all_scope() {
            test_parse_config("deps: [':task']", load_config_from_code);
        }

        #[test]
        #[should_panic(expected = "a target field is required")]
        fn errors_if_using_object_with_no_target() {
            test_parse_config(
                r"
deps:
  - args: a b c
",
                load_config_from_code,
            );
        }
    }

    mod inputs {
        use super::*;

        #[test]
        fn supports_path_patterns() {
            let config = test_parse_config(
                r"
inputs:
  - /ws/path
  - '/ws/glob/**/*'
  - '/!ws/glob/**/*'
  - proj/path
  - 'proj/glob/{a,b,c}'
  - '!proj/glob/{a,b,c}'
",
                load_config_from_code,
            );

            assert_eq!(
                config.inputs.unwrap(),
                vec![
                    Input::File(stub_file_input("/ws/path")),
                    Input::Glob(stub_glob_input("/ws/glob/**/*")),
                    Input::Glob(stub_glob_input("!/ws/glob/**/*")),
                    Input::File(stub_file_input("proj/path")),
                    Input::Glob(stub_glob_input("proj/glob/{a,b,c}")),
                    Input::Glob(stub_glob_input("!proj/glob/{a,b,c}")),
                ]
            );
        }

        #[test]
        fn supports_path_protocols() {
            let config = test_parse_config(
                r"
inputs:
  - file:///ws/path?match=a|b|c
  - 'glob:///ws/glob/**/*'
  - 'glob:///!ws/glob/**/*'
  - file://proj/path?optional
  - 'glob://proj/glob/{a,b,c}'
  - 'glob://!proj/glob/{a,b,c}?cache=false'
",
                load_config_from_code,
            );

            assert_eq!(
                config.inputs.unwrap(),
                vec![
                    Input::File({
                        let mut inner = stub_file_input("/ws/path");
                        inner.content = Some(RegexSetting::new("a|b|c").unwrap());
                        inner
                    }),
                    Input::Glob(stub_glob_input("/ws/glob/**/*")),
                    Input::Glob(stub_glob_input("!/ws/glob/**/*")),
                    Input::File({
                        let mut inner = stub_file_input("proj/path");
                        inner.optional = Some(true);
                        inner
                    }),
                    Input::Glob(stub_glob_input("proj/glob/{a,b,c}")),
                    Input::Glob({
                        let mut inner = stub_glob_input("!proj/glob/{a,b,c}");
                        inner.cache = false;
                        inner
                    }),
                ]
            );
        }

        #[test]
        fn supports_path_objects() {
            let config = test_parse_config(
                r"
inputs:
  - file: '/ws/path'
    content: 'a|b|c'
  - glob: '/ws/glob/**/*'
  - glob: '/!ws/glob/**/*'
  - file:  proj/path
    optional: true
  - glob: 'proj/glob/{a,b,c}'
  - glob: '!proj/glob/{a,b,c}'
    cache: false
",
                load_config_from_code,
            );

            assert_eq!(
                config.inputs.unwrap(),
                vec![
                    Input::File({
                        let mut inner = stub_file_input("/ws/path");
                        inner.content = Some(RegexSetting::new("a|b|c").unwrap());
                        inner
                    }),
                    Input::Glob(stub_glob_input("/ws/glob/**/*")),
                    Input::Glob(stub_glob_input("!/ws/glob/**/*")),
                    Input::File({
                        let mut inner = stub_file_input("proj/path");
                        inner.optional = Some(true);
                        inner
                    }),
                    Input::Glob(stub_glob_input("proj/glob/{a,b,c}")),
                    Input::Glob({
                        let mut inner = stub_glob_input("!proj/glob/{a,b,c}");
                        inner.cache = false;
                        inner
                    }),
                ]
            );
        }

        #[test]
        fn supports_mixing_path_formats() {
            let config = test_parse_config(
                r"
inputs:
  - file: '/ws/path'
    content: 'a|b|c'
  - '/ws/glob/**/*'
  - 'glob:///!ws/glob/**/*'
  - 'file://proj/path?optional'
  - 'proj/glob/{a,b,c}'
  - glob: '!proj/glob/{a,b,c}'
    cache: false
",
                load_config_from_code,
            );

            assert_eq!(
                config.inputs.unwrap(),
                vec![
                    Input::File({
                        let mut inner = stub_file_input("/ws/path");
                        inner.content = Some(RegexSetting::new("a|b|c").unwrap());
                        inner
                    }),
                    Input::Glob(stub_glob_input("/ws/glob/**/*")),
                    Input::Glob(stub_glob_input("!/ws/glob/**/*")),
                    Input::File({
                        let mut inner = stub_file_input("proj/path");
                        inner.optional = Some(true);
                        inner
                    }),
                    Input::Glob(stub_glob_input("proj/glob/{a,b,c}")),
                    Input::Glob({
                        let mut inner = stub_glob_input("!proj/glob/{a,b,c}");
                        inner.cache = false;
                        inner
                    }),
                ]
            );
        }

        #[test]
        fn supports_env_vars() {
            let config = test_parse_config(
                r"
inputs:
  - $FOO_BAR
  - $FOO_*
  - file/path
",
                load_config_from_code,
            );

            assert_eq!(
                config.inputs.unwrap(),
                vec![
                    Input::EnvVar("FOO_BAR".into()),
                    Input::EnvVarGlob("FOO_*".into()),
                    Input::File(stub_file_input("file/path")),
                ]
            );
        }

        #[test]
        fn supports_file_group_formats() {
            let config = test_parse_config(
                r"
inputs:
  - group://sources
  - group://sources?as=dirs
  - group: 'tests'
  - group: 'tests'
    as: 'files'
",
                load_config_from_code,
            );

            assert_eq!(
                config.inputs.unwrap(),
                vec![
                    Input::FileGroup(FileGroupInput {
                        group: Id::raw("sources"),
                        ..Default::default()
                    }),
                    Input::FileGroup(FileGroupInput {
                        group: Id::raw("sources"),
                        format: FileGroupInputFormat::Dirs,
                    }),
                    Input::FileGroup(FileGroupInput {
                        group: Id::raw("tests"),
                        ..Default::default()
                    }),
                    Input::FileGroup(FileGroupInput {
                        group: Id::raw("tests"),
                        format: FileGroupInputFormat::Files,
                    })
                ]
            );
        }

        #[test]
        fn supports_project_sources_formats() {
            let config = test_parse_config(
                r"
inputs:
  - project://a
  - project://b?filter=src/**
  - project: c
    fileGroup: sources
  - project: ^
",
                load_config_from_code,
            );

            assert_eq!(
                config.inputs.unwrap(),
                vec![
                    Input::Project(ProjectInput {
                        project: "a".into(),
                        ..Default::default()
                    }),
                    Input::Project(ProjectInput {
                        project: "b".into(),
                        filter: vec!["src/**".into()],
                        ..Default::default()
                    }),
                    Input::Project(ProjectInput {
                        project: "c".into(),
                        group: Some(Id::raw("sources")),
                        ..Default::default()
                    }),
                    Input::Project(ProjectInput {
                        project: "^".into(),
                        ..Default::default()
                    }),
                ]
            );
        }
    }

    mod outputs {
        use super::*;

        #[test]
        fn supports_path_patterns() {
            let config = test_parse_config(
                r"
outputs:
  - /ws/path
  - '/ws/glob/**/*'
  # - '/!ws/glob/**/*'
  - proj/path
  - 'proj/glob/{a,b,c}'
  # - '!proj/glob/{a,b,c}'
",
                load_config_from_code,
            );

            assert_eq!(
                config.outputs.unwrap(),
                vec![
                    Output::File(stub_file_output("/ws/path")),
                    Output::Glob(stub_glob_output("/ws/glob/**/*")),
                    // Output::WorkspaceGlob("!ws/glob/**/*".into()),
                    Output::File(stub_file_output("proj/path")),
                    Output::Glob(stub_glob_output("proj/glob/{a,b,c}")),
                    // Output::ProjectGlob("!proj/glob/{a,b,c}".into()),
                ]
            );
        }

        #[test]
        #[should_panic(expected = "environment variable is not supported by itself")]
        fn errors_on_env_var() {
            test_parse_config(
                r"
outputs:
  - $FOO_BAR
  - file/path
",
                load_config_from_code,
            );
        }
    }

    mod platform {
        use super::*;

        #[test]
        fn supports_variant() {
            let config = test_parse_config("platform: rust", load_config_from_code);

            assert_eq!(config.platform, PlatformType::Rust);
        }

        #[test]
        #[should_panic(
            expected = "Failed to parse TaskConfig. platform: unknown variant `perl`, expected one of `bun`, `deno`, `node`, `python`, `rust`, `system`, `unknown`"
        )]
        fn errors_on_invalid_variant() {
            test_parse_config("platform: perl", load_config_from_code);
        }
    }

    mod type_of {
        use super::*;

        #[test]
        fn supports_variant() {
            let config = test_parse_config("type: build", load_config_from_code);

            assert_eq!(config.type_of, Some(TaskType::Build));
        }

        #[test]
        #[should_panic(
            expected = "unknown variant `cache`, expected one of `build`, `run`, `test`"
        )]
        fn errors_on_invalid_variant() {
            test_parse_config("type: cache", load_config_from_code);
        }
    }

    mod options {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_parse_config("{}", load_config_from_code);
            let opts = config.options;

            assert_eq!(opts.affected_files, None);
            assert_eq!(opts.env_file, None);
        }

        #[test]
        fn can_set_options() {
            let config = test_parse_config(
                r"
options:
  cache: false
  runDepsInParallel: false
  mergeDeps: replace
  outputStyle: stream
",
                load_config_from_code,
            );
            let opts = config.options;

            assert_eq!(opts.cache, Some(TaskOptionCache::Enabled(false)));
            assert_eq!(opts.run_deps_in_parallel, Some(false));
            assert_eq!(opts.merge_deps, Some(TaskMergeStrategy::Replace));
            assert_eq!(opts.output_style, Some(TaskOutputStyle::Stream));
        }

        mod affected_files {
            use super::*;
            use moon_config::TaskOptionAffectedFiles;

            #[test]
            fn can_use_true() {
                let config = test_parse_config(
                    r"
options:
  affectedFiles: true
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.affected_files,
                    Some(TaskOptionAffectedFiles::Enabled(true))
                );
            }

            #[test]
            fn can_use_false() {
                let config = test_parse_config(
                    r"
options:
  affectedFiles: false
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.affected_files,
                    Some(TaskOptionAffectedFiles::Enabled(false))
                );
            }

            #[test]
            fn can_set_args() {
                let config = test_parse_config(
                    r"
options:
  affectedFiles: args
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.affected_files,
                    Some(TaskOptionAffectedFiles::Args)
                );
            }

            #[test]
            fn can_set_env() {
                let config = test_parse_config(
                    r"
options:
  affectedFiles: env
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.affected_files,
                    Some(TaskOptionAffectedFiles::Env)
                );
            }

            #[test]
            #[should_panic(expected = "expected `args`, `env`, or a boolean")]
            fn errors_on_invalid_variant() {
                test_parse_config(
                    r"
options:
  affectedFiles: other
",
                    load_config_from_code,
                );
            }
        }

        mod cache {
            use super::*;

            #[test]
            fn can_set_local() {
                let config = test_parse_config(
                    r"
options:
  cache: local
",
                    load_config_from_code,
                );
                let opts = config.options;

                assert_eq!(opts.cache, Some(TaskOptionCache::Local));
            }

            #[test]
            fn can_set_remote() {
                let config = test_parse_config(
                    r"
options:
  cache: remote
",
                    load_config_from_code,
                );
                let opts = config.options;

                assert_eq!(opts.cache, Some(TaskOptionCache::Remote));
            }
        }

        mod env_file {
            use super::*;
            use moon_config::TaskOptionEnvFile;

            #[test]
            fn can_use_true() {
                let config = test_parse_config(
                    r"
options:
  envFile: true
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.env_file,
                    Some(TaskOptionEnvFile::Enabled(true))
                );
            }

            #[test]
            fn can_use_false() {
                let config = test_parse_config(
                    r"
options:
  envFile: false
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.env_file,
                    Some(TaskOptionEnvFile::Enabled(false))
                );
            }

            #[test]
            fn can_set_project_path() {
                let config = test_parse_config(
                    r"
options:
  envFile: .env.file
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.env_file,
                    Some(TaskOptionEnvFile::File(FilePath(".env.file".into())))
                );
            }

            #[test]
            fn can_set_workspace_path() {
                let config = test_parse_config(
                    r"
options:
  envFile: /.env.file
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.env_file,
                    Some(TaskOptionEnvFile::File(FilePath("/.env.file".into())))
                );
            }

            #[test]
            fn can_set_a_list() {
                let config = test_parse_config(
                    r"
options:
  envFile: [.env.file, /.env.shared]
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.env_file,
                    Some(TaskOptionEnvFile::Files(vec![
                        FilePath(".env.file".into()),
                        FilePath("/.env.shared".into())
                    ]))
                );
            }

            #[test]
            #[should_panic(expected = "expected a boolean, a file path, or a list of file paths")]
            fn errors_on_glob() {
                test_parse_config(
                    r"
options:
  envFile: .env.*
",
                    load_config_from_code,
                );
            }

            //             #[test]
            //             #[should_panic(expected = "environment variables are not supported")]
            //             fn errors_on_env_var() {
            //                 test_parse_config(
            //                     r"
            // options:
            //   envFile: $ENV_VAR
            // ",
            //                     load_config_from_code,
            //                 );
            //             }
        }

        mod interactive {
            use super::*;

            #[test]
            #[should_panic(expected = "an interactive task cannot be persistent")]
            fn errors_if_persistent() {
                test_parse_config(
                    r"
options:
  interactive: true
  persistent: true
",
                    load_config_from_code,
                );
            }
        }

        mod os {
            use super::*;
            use moon_config::TaskOperatingSystem;

            #[test]
            fn can_set_one() {
                let config = test_parse_config(
                    r"
options:
  os: windows
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.os,
                    Some(OneOrMany::One(TaskOperatingSystem::Windows))
                );
            }

            #[test]
            fn can_set_many() {
                let config = test_parse_config(
                    r"
options:
  os: [linux, macos]
",
                    load_config_from_code,
                );

                assert_eq!(
                    config.options.os,
                    Some(OneOrMany::Many(vec![
                        TaskOperatingSystem::Linux,
                        TaskOperatingSystem::Macos
                    ]))
                );
            }

            #[test]
            #[should_panic(expected = "expected a single value, or a list of values")]
            fn errors_for_unknown() {
                test_parse_config(
                    r"
options:
  os: unknown
",
                    load_config_from_code,
                );
            }
        }
    }

    mod pkl {
        use super::*;
        use moon_config::*;
        use starbase_sandbox::locate_fixture;

        #[test]
        fn loads_pkl() {
            let config = test_config(locate_fixture("pkl"), |root| {
                load_config_from_file(&root.join("task.pkl"))
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
                    local: Some(true),
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
                    platform: PlatformType::Bun,
                    preset: Some(TaskPreset::Server),
                    type_of: Some(TaskType::Build),
                    ..Default::default()
                }
            );
        }
    }
}
