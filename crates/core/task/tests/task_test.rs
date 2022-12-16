use moon_config::{TaskCommandArgs, TaskConfig, TaskOptionEnvFileConfig, TaskOptionsConfig};
use moon_task::{Target, Task, TaskOptions};
use moon_test_utils::{create_sandbox, get_fixtures_path};
use moon_utils::{glob, string_vec};
use rustc_hash::FxHashSet;
use std::env;

pub fn create_task(config: TaskConfig) -> Task {
    Task::from_config(Target::new("project", "task").unwrap(), &config).unwrap()
}

mod from_config {
    use super::*;
    use moon_config::{TaskMergeStrategy, TaskOutputStyle};

    #[test]
    fn sets_defaults() {
        let task =
            Task::from_config(Target::new("foo", "test").unwrap(), &TaskConfig::default()).unwrap();

        assert_eq!(task.inputs, string_vec!["**/*"]);
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
                    env_file: Some(TaskOptionEnvFileConfig::Enabled(true)),
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
    use super::*;
    use moon_config::TaskMergeStrategy;

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
        let mut task = create_task(TaskConfig {
            inputs: Some(string_vec!["$FOO"]),
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
            inputs: Some(string_vec!["$BAR"]),
            ..TaskConfig::default()
        });

        task.input_vars.insert("BAR".into());

        assert!(!task.is_affected(&FxHashSet::default()).unwrap());
    }

    #[test]
    fn returns_false_if_var_empty() {
        let mut task = create_task(TaskConfig {
            inputs: Some(string_vec!["$BAZ"]),
            ..TaskConfig::default()
        });

        task.input_vars.insert("BAZ".into());

        env::set_var("BAZ", "");

        assert!(!task.is_affected(&FxHashSet::default()).unwrap());

        env::remove_var("BAZ");
    }

    #[test]
    fn returns_true_if_matches_file() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let mut task = create_task(TaskConfig {
            inputs: Some(string_vec!["file.ts"]),
            ..TaskConfig::default()
        });

        task.input_paths.insert(project_root.join("file.ts"));

        let mut set = FxHashSet::default();
        set.insert(project_root.join("file.ts"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_true_if_matches_glob() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let mut task = create_task(TaskConfig {
            inputs: Some(string_vec!["file.*"]),
            ..TaskConfig::default()
        });

        task.input_globs
            .insert(glob::normalize(project_root.join("file.*")).unwrap());

        let mut set = FxHashSet::default();
        set.insert(project_root.join("file.ts"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_true_when_referencing_root_files() {
        let workspace_root = get_fixtures_path("base");
        let mut task = create_task(TaskConfig {
            inputs: Some(string_vec!["/package.json"]),
            ..TaskConfig::default()
        });

        task.input_paths.insert(workspace_root.join("package.json"));

        let mut set = FxHashSet::default();
        set.insert(workspace_root.join("package.json"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_false_if_outside_project() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let mut task = create_task(TaskConfig {
            inputs: Some(string_vec!["file.ts"]),
            ..TaskConfig::default()
        });

        task.input_paths.insert(project_root.join("file.ts"));

        let mut set = FxHashSet::default();
        set.insert(workspace_root.join("base/other/outside.ts"));

        assert!(!task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_false_if_no_match() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let mut task = create_task(TaskConfig {
            inputs: Some(string_vec!["file.ts", "src/*"]),
            ..TaskConfig::default()
        });

        task.input_paths.insert(project_root.join("file.ts"));
        task.input_globs
            .insert(glob::normalize(project_root.join("src/*")).unwrap());

        let mut set = FxHashSet::default();
        set.insert(project_root.join("another.rs"));

        assert!(!task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_true_for_env_file() {
        let sandbox = create_sandbox("base");
        sandbox.create_file("files-and-dirs/.env", "");

        let project_root = sandbox.path().join("files-and-dirs");
        let mut task = create_task(TaskConfig {
            options: TaskOptionsConfig {
                env_file: Some(TaskOptionEnvFileConfig::Enabled(true)),
                ..TaskOptionsConfig::default()
            },
            ..TaskConfig::default()
        });

        task.input_paths.insert(project_root.join(".env"));

        let mut set = FxHashSet::default();
        set.insert(project_root.join(".env"));

        assert!(task.is_affected(&set).unwrap());
    }
}
