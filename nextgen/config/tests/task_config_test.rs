mod utils;

use moon_config2::{
    PlatformType, TaskCommandArgs, TaskConfig, TaskMergeStrategy, TaskOutputStyle, TaskType,
};
use moon_target::Target;
use utils::*;

mod task_config {
    use super::*;

    #[test]
    #[should_panic(
        expected = "unknown field `unknown`, expected one of `command`, `args`, `deps`, `env`, `inputs`, `local`, `outputs`, `options`, `platform`, `type`"
    )]
    fn error_unknown_field() {
        test_parse_config("unknown: 123", |code| TaskConfig::parse(code));
    }

    #[test]
    fn loads_defaults() {
        let config = test_parse_config("{}", |code| TaskConfig::parse(code));

        assert_eq!(config.command, TaskCommandArgs::None);
        assert_eq!(config.args, TaskCommandArgs::None);
        assert_eq!(config.type_of, None);
    }

    mod command {
        use super::*;

        #[test]
        #[should_panic(expected = "expected a string or a sequence of strings")]
        fn errors_on_invalid_type() {
            test_parse_config("command: 123", |code| TaskConfig::parse(code));
        }

        #[test]
        #[should_panic(expected = "a command is required; use \"noop\" otherwise")]
        fn errors_for_empty_string() {
            test_parse_config("command: ''", |code| TaskConfig::parse(code));
        }

        #[test]
        #[should_panic(expected = "a command is required; use \"noop\" otherwise")]
        fn errors_for_empty_list() {
            test_parse_config("command: []", |code| TaskConfig::parse(code));
        }

        #[test]
        #[should_panic(expected = "a command is required; use \"noop\" otherwise")]
        fn errors_for_empty_list_arg() {
            test_parse_config("command: ['']", |code| TaskConfig::parse(code));
        }

        #[test]
        fn parses_string() {
            let config = test_parse_config("command: bin", |code| TaskConfig::parse(code));

            assert_eq!(config.command, TaskCommandArgs::String("bin".into()));
        }

        #[test]
        fn parses_list() {
            let config = test_parse_config("command: [bin]", |code| TaskConfig::parse(code));

            assert_eq!(
                config.command,
                TaskCommandArgs::Sequence(vec!["bin".into()])
            );
        }
    }

    mod args {
        use super::*;

        #[test]
        fn parses_string() {
            let config = test_parse_config("args: bin", |code| TaskConfig::parse(code));

            assert_eq!(config.args, TaskCommandArgs::String("bin".into()));
        }

        #[test]
        fn parses_list() {
            let config = test_parse_config("args: [bin]", |code| TaskConfig::parse(code));

            assert_eq!(config.args, TaskCommandArgs::Sequence(vec!["bin".into()]));
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
                |code| TaskConfig::parse(code),
            );

            assert_eq!(
                config.args,
                TaskCommandArgs::Sequence(vec![
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
                |code| TaskConfig::parse(code),
            );

            assert_eq!(
                config.deps,
                vec![
                    Target::parse("task").unwrap(),
                    Target::parse("project:task").unwrap(),
                    Target::parse("^:task").unwrap(),
                    Target::parse("~:task").unwrap()
                ]
            );
        }

        #[test]
        #[should_panic(expected = "Invalid target ~:bad target")]
        fn errors_on_invalid_format() {
            test_parse_config("deps: ['bad target']", |code| TaskConfig::parse(code));
        }

        #[test]
        #[should_panic(expected = "target scope not supported as a task dependency")]
        fn errors_on_all_scope() {
            test_parse_config("deps: [':task']", |code| TaskConfig::parse(code));
        }

        #[test]
        #[should_panic(expected = "target scope not supported as a task dependency")]
        fn errors_on_tag_scope() {
            test_parse_config("deps: ['#tag:task']", |code| TaskConfig::parse(code));
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
                |code| TaskConfig::parse(code),
            );

            // assert_eq!(
            //     config.inputs,
            //     vec![
            //         PortablePath::WorkspaceFile(FilePath("ws/path".into())),
            //         PortablePath::WorkspaceGlob(GlobPath("ws/glob/**/*".into())),
            //         PortablePath::WorkspaceGlob(GlobPath("!ws/glob/**/*".into())),
            //         PortablePath::ProjectFile(FilePath("proj/path".into())),
            //         PortablePath::ProjectGlob(GlobPath("proj/glob/{a,b,c}".into())),
            //         PortablePath::ProjectGlob(GlobPath("!proj/glob/{a,b,c}".into())),
            //     ]
            // );
            assert_eq!(
                config.inputs.unwrap(),
                vec![
                    "/ws/path".to_owned(),
                    "/ws/glob/**/*".to_owned(),
                    "/!ws/glob/**/*".to_owned(),
                    "proj/path".to_owned(),
                    "proj/glob/{a,b,c}".to_owned(),
                    "!proj/glob/{a,b,c}".to_owned(),
                ]
            );
        }

        #[test]
        fn supports_env_vars() {
            let config = test_parse_config(
                r"
inputs:
  - $FOO_BAR
  - file/path
",
                |code| TaskConfig::parse(code),
            );

            // assert_eq!(
            //     config.inputs,
            //     vec![
            //         PortablePath::EnvVar("FOO_BAR".into()),
            //         PortablePath::ProjectFile(FilePath("file/path".into())),
            //     ]
            // );
            assert_eq!(
                config.inputs.unwrap(),
                vec!["$FOO_BAR".to_owned(), "file/path".to_owned(),]
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
  - '/!ws/glob/**/*'
  - proj/path
  - 'proj/glob/{a,b,c}'
  - '!proj/glob/{a,b,c}'
",
                |code| TaskConfig::parse(code),
            );

            // assert_eq!(
            //     config.outputs,
            //     vec![
            //         PortablePath::WorkspaceFile(FilePath("ws/path".into())),
            //         PortablePath::WorkspaceGlob(GlobPath("ws/glob/**/*".into())),
            //         PortablePath::WorkspaceGlob(GlobPath("!ws/glob/**/*".into())),
            //         PortablePath::ProjectFile(FilePath("proj/path".into())),
            //         PortablePath::ProjectGlob(GlobPath("proj/glob/{a,b,c}".into())),
            //         PortablePath::ProjectGlob(GlobPath("!proj/glob/{a,b,c}".into())),
            //     ]
            // );
            assert_eq!(
                config.outputs.unwrap(),
                vec![
                    "/ws/path".to_owned(),
                    "/ws/glob/**/*".to_owned(),
                    "/!ws/glob/**/*".to_owned(),
                    "proj/path".to_owned(),
                    "proj/glob/{a,b,c}".to_owned(),
                    "!proj/glob/{a,b,c}".to_owned(),
                ]
            );
        }

        //         #[test]
        //         #[should_panic(expected = "environment variables are not supported here")]
        //         fn errors_on_env_var() {
        //             test_parse_config(
        //                 r"
        // outputs:
        //   - $FOO_BAR
        //   - file/path
        // ",
        //                 |code| TaskConfig::parse(code),
        //             );
        //         }
    }

    mod platform {
        use super::*;

        #[test]
        fn supports_variant() {
            let config = test_parse_config("platform: rust", |code| TaskConfig::parse(code));

            assert_eq!(config.platform, PlatformType::Rust);
        }

        #[test]
        #[should_panic(
            expected = "unknown variant `perl`, expected one of `deno`, `node`, `rust`, `system`, `unknown`"
        )]
        fn errors_on_invalid_variant() {
            test_parse_config("platform: perl", |code| TaskConfig::parse(code));
        }
    }

    mod type_of {
        use super::*;

        #[test]
        fn supports_variant() {
            let config = test_parse_config("type: build", |code| TaskConfig::parse(code));

            assert_eq!(config.type_of, Some(TaskType::Build));
        }

        #[test]
        #[should_panic(
            expected = "unknown variant `cache`, expected one of `build`, `run`, `test`"
        )]
        fn errors_on_invalid_variant() {
            test_parse_config("type: cache", |code| TaskConfig::parse(code));
        }
    }

    mod options {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_parse_config("{}", |code| TaskConfig::parse(code));
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
                |code| TaskConfig::parse(code),
            );
            let opts = config.options;

            assert_eq!(opts.cache, Some(false));
            assert_eq!(opts.run_deps_in_parallel, Some(false));
            assert_eq!(opts.merge_deps, Some(TaskMergeStrategy::Replace));
            assert_eq!(opts.output_style, Some(TaskOutputStyle::Stream));
        }

        mod affected_files {
            use super::*;
            use moon_config2::TaskOptionAffectedFiles;

            #[test]
            fn can_use_true() {
                let config = test_parse_config(
                    r"
options:
  affectedFiles: true
",
                    |code| TaskConfig::parse(code),
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
                    |code| TaskConfig::parse(code),
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
                    |code| TaskConfig::parse(code),
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
                    |code| TaskConfig::parse(code),
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
                    |code| TaskConfig::parse(code),
                );
            }
        }

        mod env_file {
            use super::*;
            use moon_config2::TaskOptionEnvFile;

            #[test]
            fn can_use_true() {
                let config = test_parse_config(
                    r"
options:
  envFile: true
",
                    |code| TaskConfig::parse(code),
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
                    |code| TaskConfig::parse(code),
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
                    |code| TaskConfig::parse(code),
                );

                assert_eq!(
                    config.options.env_file,
                    Some(TaskOptionEnvFile::File(".env.file".to_owned()))
                );
            }

            #[test]
            fn can_set_workspace_path() {
                let config = test_parse_config(
                    r"
options:
  envFile: /.env.file
",
                    |code| TaskConfig::parse(code),
                );

                assert_eq!(
                    config.options.env_file,
                    Some(TaskOptionEnvFile::File("/.env.file".to_owned()))
                );
            }

            #[test]
            #[should_panic(expected = "globs are not supported")]
            fn errors_on_glob() {
                test_parse_config(
                    r"
options:
  envFile: .env.*
",
                    |code| TaskConfig::parse(code),
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
            //                     |code| TaskConfig::parse(code),
            //                 );
            //             }
        }
    }
}
