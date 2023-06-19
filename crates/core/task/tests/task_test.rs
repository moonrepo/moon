use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{
    FilePath, InputPath, TaskCommandArgs, TaskConfig, TaskMergeStrategy, TaskOptionEnvFile,
    TaskOptionsConfig, TaskOutputStyle,
};
use moon_target::Target;
use moon_task::{Task, TaskFlag, TaskOptions};
use moon_test_utils::create_sandbox;
use moon_utils::string_vec;
use rustc_hash::FxHashSet;
use std::env;
use std::str::FromStr;

pub fn create_task(config: TaskConfig) -> Task {
    Task::from_config(Target::new("project", "task").unwrap(), &config).unwrap()
}

mod from_config {
    use super::*;

    #[test]
    fn sets_defaults() {
        let task =
            Task::from_config(Target::new("foo", "test").unwrap(), &TaskConfig::default()).unwrap();

        assert!(task.inputs.is_empty());
        assert_eq!(task.log_target, "moon:project:foo:test");
        assert_eq!(task.target, Target::new("foo", "test").unwrap());
        assert_eq!(
            task.options,
            TaskOptions {
                affected_files: None,
                cache: true,
                env_file: None,
                merge_args: TaskMergeStrategy::Append,
                merge_deps: TaskMergeStrategy::Append,
                merge_env: TaskMergeStrategy::Append,
                merge_inputs: TaskMergeStrategy::Append,
                merge_outputs: TaskMergeStrategy::Append,
                output_style: None,
                persistent: false,
                retry_count: 0,
                run_deps_in_parallel: true,
                run_in_ci: true,
                run_from_workspace_root: false,
                shell: true,
            }
        )
    }

    #[test]
    fn changes_options_if_local() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                local: Some(true),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(
            task.options,
            TaskOptions {
                cache: false,
                output_style: Some(TaskOutputStyle::Stream),
                persistent: true,
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
                command: TaskCommandArgs::String("dev".to_owned()),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(
            task.options,
            TaskOptions {
                cache: false,
                output_style: Some(TaskOutputStyle::Stream),
                persistent: true,
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
                local: Some(true),
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
                persistent: true,
                run_in_ci: false,
                ..TaskOptions::default()
            }
        )
    }

    #[test]
    fn can_override_local_persistent() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                local: Some(true),
                options: TaskOptionsConfig {
                    persistent: Some(false),
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
                output_style: Some(TaskOutputStyle::Stream),
                persistent: false,
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
                env_file: Some(FilePath(".env".to_owned())),
                ..TaskOptions::default()
            }
        )
    }

    #[test]
    fn handles_command_string() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                command: TaskCommandArgs::String("foo --bar".to_owned()),
                args: TaskCommandArgs::Sequence(string_vec!["--baz"]),
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
                command: TaskCommandArgs::Sequence(string_vec!["foo", "--bar"]),
                args: TaskCommandArgs::String("--baz".to_owned()),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(task.command, "foo".to_owned());
        assert_eq!(task.args, string_vec!["--bar", "--baz"]);
    }

    #[test]
    fn sets_inputs() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                inputs: Some(vec![
                    InputPath::from_str("foo").unwrap(),
                    InputPath::from_str("bar").unwrap(),
                ]),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(
            task.inputs,
            vec![
                InputPath::ProjectFile("foo".into()),
                InputPath::ProjectFile("bar".into())
            ]
        );
    }

    #[test]
    fn sets_empty_inputs() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                inputs: Some(vec![]),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(task.inputs, Vec::<InputPath>::new());
    }

    #[test]
    fn sets_global_and_local_inputs() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                global_inputs: vec![InputPath::from_str("global").unwrap()],
                inputs: Some(vec![InputPath::from_str("local").unwrap()]),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert_eq!(
            task.global_inputs,
            vec![InputPath::ProjectFile("global".into())]
        );
        assert_eq!(task.inputs, vec![InputPath::ProjectFile("local".into())]);
    }

    #[test]
    fn sets_empty_flag() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                inputs: Some(vec![]),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert!(task.flags.contains(&TaskFlag::NoInputs));
    }

    #[test]
    fn doesnt_set_empty_flag_when_inputs() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                inputs: Some(vec![InputPath::from_str("foo").unwrap()]),
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert!(!task.flags.contains(&TaskFlag::NoInputs));
    }

    #[test]
    fn doesnt_set_empty_flag_when_none() {
        let task = Task::from_config(
            Target::new("foo", "test").unwrap(),
            &TaskConfig {
                inputs: None,
                ..TaskConfig::default()
            },
        )
        .unwrap();

        assert!(!task.flags.contains(&TaskFlag::NoInputs));
    }
}

mod merge {
    use super::*;

    #[test]
    fn merges_command_string() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            command: TaskCommandArgs::String("foo --bar".to_owned()),
            args: TaskCommandArgs::Sequence(string_vec!["--baz"]),
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
            command: TaskCommandArgs::Sequence(string_vec!["foo", "--bar"]),
            args: TaskCommandArgs::String("--baz".to_owned()),
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
            command: TaskCommandArgs::String("foo --after".to_owned()),
            args: TaskCommandArgs::String("--post".to_owned()),
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
            command: TaskCommandArgs::String("foo --before".to_owned()),
            args: TaskCommandArgs::String("--pre".to_owned()),
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
            command: TaskCommandArgs::String("foo --new".to_owned()),
            args: TaskCommandArgs::String("--hot".to_owned()),
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
            args: TaskCommandArgs::String("--a".to_owned()),
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
            args: TaskCommandArgs::Sequence(string_vec!["--b"]),
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
            command: TaskCommandArgs::String("foo --r".to_owned()),
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

    #[test]
    fn can_overwrite_to_empty_inputs() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            inputs: vec![InputPath::from_str("**/*").unwrap()],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            inputs: Some(vec![]),
            options: TaskOptionsConfig {
                merge_inputs: Some(TaskMergeStrategy::Replace),
                ..TaskOptionsConfig::default()
            },
            ..TaskConfig::default()
        })
        .unwrap();

        assert_eq!(task.inputs, Vec::<InputPath>::new());
    }

    #[test]
    fn can_overwrite_to_empty_inputs_without_strategy() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            inputs: vec![InputPath::from_str("**/*").unwrap()],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            inputs: Some(vec![]),
            ..TaskConfig::default()
        })
        .unwrap();

        assert_eq!(task.inputs, Vec::<InputPath>::new());
    }

    #[test]
    fn sets_empty_flag() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            inputs: vec![InputPath::from_str("**/*").unwrap()],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            inputs: Some(vec![]),
            ..TaskConfig::default()
        })
        .unwrap();

        assert!(task.flags.contains(&TaskFlag::NoInputs));
    }

    #[test]
    fn doesnt_set_empty_flag_when_inputs() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            inputs: vec![InputPath::from_str("**/*").unwrap()],
            ..Task::default()
        };

        task.merge(&TaskConfig {
            inputs: Some(vec![InputPath::from_str("foo").unwrap()]),
            ..TaskConfig::default()
        })
        .unwrap();

        assert!(!task.flags.contains(&TaskFlag::NoInputs));
    }

    #[test]
    fn doesnt_overwrite_parent_flag_when_none() {
        let mut task = Task {
            command: "cmd".to_owned(),
            args: string_vec!["--arg"],
            inputs: vec![InputPath::from_str("**/*").unwrap()],
            flags: FxHashSet::from_iter([TaskFlag::NoInputs]),
            ..Task::default()
        };

        task.merge(&TaskConfig {
            inputs: Some(vec![]),
            ..TaskConfig::default()
        })
        .unwrap();

        assert!(task.flags.contains(&TaskFlag::NoInputs));
    }
}

mod is_affected {
    use super::*;

    #[test]
    fn returns_true_if_var_truthy() {
        let mut task = create_task(TaskConfig {
            inputs: Some(vec![InputPath::from_str("$FOO").unwrap()]),
            ..TaskConfig::default()
        });

        task.input_vars.insert("FOO".into());

        env::set_var("FOO", "foo");

        assert!(task.is_affected(&FxHashSet::default()).unwrap());

        env::remove_var("FOO");
    }

    #[test]
    fn returns_false_if_var_missing() {
        let mut task = create_task(TaskConfig {
            inputs: Some(vec![InputPath::from_str("$BAR").unwrap()]),
            ..TaskConfig::default()
        });

        task.input_vars.insert("BAR".into());

        assert!(!task.is_affected(&FxHashSet::default()).unwrap());
    }

    #[test]
    fn returns_false_if_var_empty() {
        let mut task = create_task(TaskConfig {
            inputs: Some(vec![InputPath::from_str("$BAZ").unwrap()]),
            ..TaskConfig::default()
        });

        task.input_vars.insert("BAZ".into());

        env::set_var("BAZ", "");

        assert!(!task.is_affected(&FxHashSet::default()).unwrap());

        env::remove_var("BAZ");
    }

    #[test]
    fn returns_true_if_matches_file() {
        let project_source = WorkspaceRelativePathBuf::from("files-and-dirs");
        let mut task = create_task(TaskConfig {
            inputs: Some(vec![InputPath::from_str("file.ts").unwrap()]),
            ..TaskConfig::default()
        });

        task.input_paths.insert(project_source.join("file.ts"));

        let mut set = FxHashSet::default();
        set.insert(project_source.join("file.ts"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_true_if_matches_glob() {
        let project_source = WorkspaceRelativePathBuf::from("files-and-dirs");
        let mut task = create_task(TaskConfig {
            inputs: Some(vec![InputPath::from_str("file.*").unwrap()]),
            ..TaskConfig::default()
        });

        task.input_globs.insert("files-and-dirs/file.*".into());

        let mut set = FxHashSet::default();
        set.insert(project_source.join("file.ts"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_true_when_referencing_root_files() {
        let mut task = create_task(TaskConfig {
            inputs: Some(vec![InputPath::from_str("/package.json").unwrap()]),
            ..TaskConfig::default()
        });

        task.input_paths
            .insert(WorkspaceRelativePathBuf::from("package.json"));

        let mut set = FxHashSet::default();
        set.insert(WorkspaceRelativePathBuf::from("package.json"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_false_if_outside_project() {
        let project_source = WorkspaceRelativePathBuf::from("files-and-dirs");
        let mut task = create_task(TaskConfig {
            inputs: Some(vec![InputPath::from_str("file.ts").unwrap()]),
            ..TaskConfig::default()
        });

        task.input_paths.insert(project_source.join("file.ts"));

        let mut set = FxHashSet::default();
        set.insert(WorkspaceRelativePathBuf::from("base/other/outside.ts"));

        assert!(!task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_false_if_no_match() {
        let project_source = WorkspaceRelativePathBuf::from("files-and-dirs");
        let mut task = create_task(TaskConfig {
            inputs: Some(vec![
                InputPath::from_str("file.ts").unwrap(),
                InputPath::from_str("src/*").unwrap(),
            ]),
            ..TaskConfig::default()
        });

        task.input_paths.insert(project_source.join("file.ts"));
        task.input_globs.insert("files-and-dirs/src/*".into());

        let mut set = FxHashSet::default();
        set.insert(project_source.join("another.rs"));

        assert!(!task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_true_for_env_file() {
        let sandbox = create_sandbox("base");
        sandbox.create_file("files-and-dirs/.env", "");

        let project_source = WorkspaceRelativePathBuf::from("files-and-dirs");
        let mut task = create_task(TaskConfig {
            options: TaskOptionsConfig {
                env_file: Some(TaskOptionEnvFile::Enabled(true)),
                ..TaskOptionsConfig::default()
            },
            ..TaskConfig::default()
        });

        task.input_paths.insert(project_source.join(".env"));

        let mut set = FxHashSet::default();
        set.insert(project_source.join(".env"));

        assert!(task.is_affected(&set).unwrap());
    }
}
