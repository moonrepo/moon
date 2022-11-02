use moon_config::{TaskCommandArgs, TaskConfig, TaskOptionEnvFile, TaskOptionsConfig};
use moon_task::test::create_expanded_task;
use moon_task::{Target, Task, TaskOptions};
use moon_utils::test::{create_sandbox, get_fixtures_dir};
use moon_utils::{glob, string_vec};
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;

#[test]
#[should_panic(expected = "NoOutputGlob")]
fn errors_for_output_glob() {
    let workspace_root = get_fixtures_dir("projects");
    let project_root = workspace_root.join("basic");

    create_expanded_task(
        &workspace_root,
        &project_root,
        Some(TaskConfig {
            outputs: Some(string_vec!["some/**/glob"]),
            ..TaskConfig::default()
        }),
    )
    .unwrap();
}

mod from_config {
    use moon_config::{TaskMergeStrategy, TaskOutputStyle};

    use super::*;

    #[test]
    fn sets_defaults() {
        let task =
            Task::from_config(Target::new("foo", "test").unwrap(), &TaskConfig::default()).unwrap();

        assert_eq!(task.inputs, string_vec!["**/*"]);
        assert_eq!(task.log_target, "moon:project:foo:test");
        assert_eq!(task.target, "foo:test");
        assert_eq!(
            task.options,
            TaskOptions {
                cache: true,
                env_file: None,
                merge_args: TaskMergeStrategy::Append,
                merge_deps: TaskMergeStrategy::Append,
                merge_env: TaskMergeStrategy::Append,
                merge_inputs: TaskMergeStrategy::Append,
                merge_outputs: TaskMergeStrategy::Append,
                output_style: None,
                retry_count: 0,
                run_deps_in_parallel: true,
                run_in_ci: true,
                run_from_workspace_root: false
            }
        )
    }

    #[test]
    fn changes_options_if_local() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                local: true,
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(
            task.options,
            TaskOptions {
                cache: false,
                output_style: Some(TaskOutputStyle::Stream),
                run_in_ci: false,
                ..TaskOptions::default()
            }
        )
    }

    #[test]
    fn determines_local_from_command() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                command: Some(TaskCommandArgs::String("dev".to_owned())),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(
            task.options,
            TaskOptions {
                cache: false,
                output_style: Some(TaskOutputStyle::Stream),
                run_in_ci: false,
                ..TaskOptions::default()
            }
        )
    }

    #[test]
    fn can_override_local_output_style() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                local: true,
                options: TaskOptionsConfig {
                    output_style: Some(TaskOutputStyle::Buffer),
                    ..TaskOptionsConfig::default()
                },
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(
            task.options,
            TaskOptions {
                cache: false,
                output_style: Some(TaskOutputStyle::Buffer),
                run_in_ci: false,
                ..TaskOptions::default()
            }
        )
    }

    #[test]
    fn converts_env_file_enum() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                options: TaskOptionsConfig {
                    env_file: Some(TaskOptionEnvFile::Enabled(true)),
                    ..TaskOptionsConfig::default()
                },
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(
            task.options,
            TaskOptions {
                env_file: Some(".env".to_owned()),
                ..TaskOptions::default()
            }
        )
    }

    #[test]
    fn handles_command_string() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                command: Some(TaskCommandArgs::String("foo --bar".to_owned())),
                args: Some(TaskCommandArgs::Sequence(string_vec!["--baz"])),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(task.command, "foo".to_owned());
        assert_eq!(task.args, string_vec!["--bar", "--baz"]);
    }

    #[test]
    fn handles_command_list() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                command: Some(TaskCommandArgs::Sequence(string_vec!["foo", "--bar"])),
                args: Some(TaskCommandArgs::String("--baz".to_owned())),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(task.command, "foo".to_owned());
        assert_eq!(task.args, string_vec!["--bar", "--baz"]);
    }
}

mod merge {
    use moon_config::TaskMergeStrategy;

    use super::*;

    #[test]
    fn merges_command_string() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            command: Some(TaskCommandArgs::String("foo --bar".to_owned())),
            args: Some(TaskCommandArgs::Sequence(string_vec!["--baz"])),
            ..TaskConfig::default()
        })
        .unwrap();

        assert_eq!(task.command, "foo".to_owned());
        assert_eq!(task.args, string_vec!["--arg", "--bar", "--baz"]);
    }

    #[test]
    fn merges_command_list() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            command: Some(TaskCommandArgs::Sequence(string_vec!["foo", "--bar"])),
            args: Some(TaskCommandArgs::String("--baz".to_owned())),
            ..TaskConfig::default()
        })
        .unwrap();

        assert_eq!(task.command, "foo".to_owned());
        assert_eq!(task.args, string_vec!["--arg", "--bar", "--baz"]);
    }

    #[test]
    fn appends_command_args() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            command: Some(TaskCommandArgs::String("foo --after".to_owned())),
            args: Some(TaskCommandArgs::String("--post".to_owned())),
            options: TaskOptionsConfig {
                merge_args: Some(TaskMergeStrategy::Append),
                ..TaskOptionsConfig::default()
            },
            ..TaskConfig::default()
        })
        .unwrap();

        assert_eq!(task.args, string_vec!["--arg", "--after", "--post"]);
    }

    #[test]
    fn prepends_command_args() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            command: Some(TaskCommandArgs::String("foo --before".to_owned())),
            args: Some(TaskCommandArgs::String("--pre".to_owned())),
            options: TaskOptionsConfig {
                merge_args: Some(TaskMergeStrategy::Prepend),
                ..TaskOptionsConfig::default()
            },
            ..TaskConfig::default()
        })
        .unwrap();

        assert_eq!(task.args, string_vec!["--before", "--pre", "--arg"]);
    }

    #[test]
    fn replaces_command_args() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            command: Some(TaskCommandArgs::String("foo --new".to_owned())),
            args: Some(TaskCommandArgs::String("--hot".to_owned())),
            options: TaskOptionsConfig {
                merge_args: Some(TaskMergeStrategy::Replace),
                ..TaskOptionsConfig::default()
            },
            ..TaskConfig::default()
        })
        .unwrap();

        assert_eq!(task.args, string_vec!["--new", "--hot"]);
    }

    #[test]
    fn handles_all_strategies() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            args: Some(TaskCommandArgs::String("--a".to_owned())),
            options: TaskOptionsConfig {
                merge_args: Some(TaskMergeStrategy::Append),
                ..TaskOptionsConfig::default()
            },
            ..TaskConfig::default()
        })
        .unwrap();

        assert_eq!(task.command, "cmd".to_owned());
        assert_eq!(task.args, string_vec!["--arg", "--a"]);

        task.merge(&TaskConfig {
            args: Some(TaskCommandArgs::Sequence(string_vec!["--b"])),
            options: TaskOptionsConfig {
                merge_args: Some(TaskMergeStrategy::Prepend),
                ..TaskOptionsConfig::default()
            },
            ..TaskConfig::default()
        })
        .unwrap();

        assert_eq!(task.command, "cmd".to_owned());
        assert_eq!(task.args, string_vec!["--b", "--arg", "--a"]);

        task.merge(&TaskConfig {
            command: Some(TaskCommandArgs::String("foo --r".to_owned())),
            options: TaskOptionsConfig {
                merge_args: Some(TaskMergeStrategy::Replace),
                ..TaskOptionsConfig::default()
            },
            ..TaskConfig::default()
        })
        .unwrap();

        assert_eq!(task.command, "foo".to_owned());
        assert_eq!(task.args, string_vec!["--r"]);
    }
}

mod is_affected {
    use super::*;

    #[test]
    fn returns_true_if_var_truthy() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["$FOO"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        env::set_var("FOO", "foo");

        assert!(task.is_affected(&FxHashSet::default()).unwrap());

        env::remove_var("FOO");
    }

    #[test]
    fn returns_false_if_var_missing() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["$BAR"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        assert!(!task.is_affected(&FxHashSet::default()).unwrap());
    }

    #[test]
    fn returns_false_if_var_empty() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["$BAZ"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        env::set_var("BAZ", "");

        assert!(!task.is_affected(&FxHashSet::default()).unwrap());

        env::remove_var("BAZ");
    }

    #[test]
    fn returns_true_if_matches_file() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        let mut set = FxHashSet::default();
        set.insert(project_root.join("file.ts"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_true_if_matches_glob() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["file.*"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        let mut set = FxHashSet::default();
        set.insert(project_root.join("file.ts"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_true_when_referencing_root_files() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["/package.json"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        let mut set = FxHashSet::default();
        set.insert(workspace_root.join("package.json"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_false_if_outside_project() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        let mut set = FxHashSet::default();
        set.insert(workspace_root.join("base/other/outside.ts"));

        assert!(!task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_false_if_no_match() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["file.ts", "src/*"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        let mut set = FxHashSet::default();
        set.insert(project_root.join("another.rs"));

        assert!(!task.is_affected(&set).unwrap());
    }
}

mod expand_env {
    use super::*;
    use std::fs;

    #[test]
    #[should_panic(expected = "Error parsing line: 'FOO', error at line index: 3")]
    fn errors_on_invalid_file() {
        let fixture = create_sandbox("cases");
        let project_root = fixture.path().join("base");

        fs::write(project_root.join(".env"), "FOO").unwrap();

        create_expanded_task(
            fixture.path(),
            &project_root,
            Some(TaskConfig {
                options: TaskOptionsConfig {
                    env_file: Some(TaskOptionEnvFile::Enabled(true)),
                    ..TaskOptionsConfig::default()
                },
                ..TaskConfig::default()
            }),
        )
        .unwrap();
    }

    #[test]
    // Windows = "The system cannot find the file specified"
    // Unix = "No such file or directory"
    #[should_panic(expected = "InvalidEnvFile")]
    fn errors_on_missing_file() {
        // `expand_env` has a CI check that avoids this from crashing, so emulate it
        if moon_utils::is_ci() {
            panic!("InvalidEnvFile");
        } else {
            let fixture = create_sandbox("cases");
            let project_root = fixture.path().join("base");

            create_expanded_task(
                fixture.path(),
                &project_root,
                Some(TaskConfig {
                    options: TaskOptionsConfig {
                        env_file: Some(TaskOptionEnvFile::Enabled(true)),
                        ..TaskOptionsConfig::default()
                    },
                    ..TaskConfig::default()
                }),
            )
            .unwrap();
        }
    }

    #[test]
    fn loads_using_bool() {
        let fixture = create_sandbox("cases");
        let project_root = fixture.path().join("base");

        fs::write(project_root.join(".env"), "FOO=foo\nBAR=123").unwrap();

        let task = create_expanded_task(
            fixture.path(),
            &project_root,
            Some(TaskConfig {
                options: TaskOptionsConfig {
                    env_file: Some(TaskOptionEnvFile::Enabled(true)),
                    ..TaskOptionsConfig::default()
                },
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(
            task.env,
            FxHashMap::from([
                ("FOO".to_owned(), "foo".to_owned()),
                ("BAR".to_owned(), "123".to_owned())
            ])
        );
    }

    #[test]
    fn loads_using_custom_path() {
        let fixture = create_sandbox("cases");
        let project_root = fixture.path().join("base");

        fs::write(project_root.join(".env.production"), "FOO=foo\nBAR=123").unwrap();

        let task = create_expanded_task(
            fixture.path(),
            &project_root,
            Some(TaskConfig {
                options: TaskOptionsConfig {
                    env_file: Some(TaskOptionEnvFile::File(".env.production".to_owned())),
                    ..TaskOptionsConfig::default()
                },
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(
            task.env,
            FxHashMap::from([
                ("FOO".to_owned(), "foo".to_owned()),
                ("BAR".to_owned(), "123".to_owned())
            ])
        );
    }

    #[test]
    fn doesnt_override_other_env() {
        let fixture = create_sandbox("cases");
        let project_root = fixture.path().join("base");

        fs::write(project_root.join(".env"), "FOO=foo\nBAR=123").unwrap();

        let task = create_expanded_task(
            fixture.path(),
            &project_root,
            Some(TaskConfig {
                env: Some(FxHashMap::from([("FOO".to_owned(), "original".to_owned())])),
                options: TaskOptionsConfig {
                    env_file: Some(TaskOptionEnvFile::Enabled(true)),
                    ..TaskOptionsConfig::default()
                },
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(
            task.env,
            FxHashMap::from([
                ("FOO".to_owned(), "original".to_owned()),
                ("BAR".to_owned(), "123".to_owned())
            ])
        );
    }
}

mod expand_inputs {
    use super::*;

    #[test]
    fn filters_into_correct_types() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec![
                    "$VAR",
                    "$FOO_BAR",
                    "file.ts",
                    "folder",
                    "glob/**/*",
                    "/config.js",
                    "/*.cfg"
                ]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(
            task.input_vars,
            FxHashSet::from(["VAR".to_owned(), "FOO_BAR".to_owned()])
        );
        assert_eq!(
            task.input_paths,
            FxHashSet::from([
                project_root.join("file.ts"),
                project_root.join("folder"),
                workspace_root.join("config.js")
            ])
        );
        assert_eq!(
            task.input_globs,
            FxHashSet::from([
                glob::normalize(project_root.join("glob/**/*")).unwrap(),
                glob::normalize(workspace_root.join("*.cfg")).unwrap()
            ])
        );
    }
}
