use moon_common::Id;
use moon_config::{
    InheritedTasksManager, InputPath, OutputPath, ProjectConfig, ProjectWorkspaceConfig,
    ProjectWorkspaceInheritedTasksConfig, TaskOptionAffectedFiles, TaskOutputStyle, TaskType,
};
use moon_target::Target;
use moon_task2::Task;
use moon_task_builder::TasksBuilder;
use rustc_hash::FxHashMap;
use starbase_sandbox::create_sandbox;
use std::collections::BTreeMap;
use std::path::Path;

fn build_tasks_with_config(root: &Path, local_config: ProjectConfig) -> BTreeMap<Id, Task> {
    let id = Id::raw("project");
    let mut builder = TasksBuilder::new(&id);

    builder.load_local_tasks(&local_config);

    let global_manager = InheritedTasksManager::load(root, root.join("global")).unwrap();

    let global_config = global_manager
        .get_inherited_config(
            &local_config.platform.unwrap_or_default(),
            &local_config.language,
            &local_config.type_of,
            &local_config.tags,
        )
        .unwrap();

    builder.inherit_global_tasks(
        &global_config.config,
        Some(&local_config.workspace.inherited_tasks),
    );

    builder.build().unwrap()
}

fn build_tasks(root: &Path, config_path: &str) -> BTreeMap<Id, Task> {
    build_tasks_with_config(
        root,
        ProjectConfig::load(root, root.join(config_path)).unwrap(),
    )
}

mod tasks_builder {
    use super::*;

    #[test]
    fn loads_local_tasks() {
        let sandbox = create_sandbox("builder");
        let tasks = build_tasks(sandbox.path(), "local/moon.yml");

        let build = tasks.get("local-build").unwrap();

        assert_eq!(build.command, "local-build");
        assert_eq!(
            build.inputs,
            vec![
                InputPath::ProjectFile("abc".into()),
                InputPath::WorkspaceGlob(".moon/*.yml".into()),
            ]
        );
        assert_eq!(build.outputs, vec![OutputPath::ProjectFile("out".into())]);
        assert!(!build.flags.local);

        let run = tasks.get("local-run").unwrap();

        assert_eq!(run.command, "local-run");
        assert_eq!(
            run.inputs,
            vec![
                InputPath::ProjectFile("xyz".into()),
                InputPath::WorkspaceGlob(".moon/*.yml".into()),
            ]
        );
        assert_eq!(run.outputs, vec![]);
        assert!(run.flags.local);
    }

    #[test]
    fn inherits_global_tasks() {
        let sandbox = create_sandbox("builder");
        let tasks = build_tasks(sandbox.path(), "local/moon.yml");

        let build = tasks.get("local-build").unwrap();

        assert_eq!(build.command, "local-build");
        assert_eq!(
            build.inputs,
            vec![
                InputPath::ProjectFile("abc".into()),
                InputPath::WorkspaceGlob(".moon/*.yml".into()),
            ]
        );
        assert_eq!(build.outputs, vec![OutputPath::ProjectFile("out".into())]);
        assert!(!build.flags.local);

        let run = tasks.get("local-run").unwrap();

        assert_eq!(run.command, "local-run");
        assert_eq!(
            run.inputs,
            vec![
                InputPath::ProjectFile("xyz".into()),
                InputPath::WorkspaceGlob(".moon/*.yml".into()),
            ]
        );
        assert_eq!(run.outputs, vec![]);
        assert!(run.flags.local);
    }

    mod defaults {
        use super::*;

        #[test]
        fn sets_id() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml");
            let task = tasks.get("local-build").unwrap();

            assert_eq!(task.id, Id::raw("local-build"));
        }

        #[test]
        fn sets_target() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml");
            let task = tasks.get("local-build").unwrap();

            assert_eq!(task.target, Target::parse("project:local-build").unwrap());
        }

        #[test]
        fn type_test_by_default() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml");
            let task = tasks.get("global-test").unwrap();

            assert_eq!(task.type_of, TaskType::Test);
            assert!(task.is_test_type());
            assert!(task.should_run_in_ci());
        }

        #[test]
        fn type_run_if_local() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml");
            let task = tasks.get("global-run").unwrap();

            assert_eq!(task.type_of, TaskType::Run);
            assert!(task.is_run_type());
            assert!(!task.should_run_in_ci());
        }

        #[test]
        fn type_build_if_has_outputs() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml");
            let task = tasks.get("global-build").unwrap();

            assert_eq!(task.type_of, TaskType::Build);
            assert!(task.is_build_type());
            assert!(task.should_run_in_ci());
        }
    }

    mod command_args {
        use super::*;

        #[test]
        fn command_fallsback_to_noop() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("no-command").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, Vec::<String>::new());
        }

        #[test]
        fn command_only() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("command-only").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, Vec::<String>::new());
        }

        #[test]
        fn command_string() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("command-string").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[test]
        fn command_list() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("command-list").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[test]
        fn args_string() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("args-string").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[test]
        fn args_list() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("args-list").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[test]
        fn both_string() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("both-string").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[test]
        fn both_list() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("both-list").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[test]
        fn both_list_many() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("both-list-many").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["-qux", "--foo", "bar"]);
        }

        #[test]
        fn override_global_command() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("global-build").unwrap();

            assert_eq!(task.command, "override-bin");
            assert_eq!(task.args, vec!["--with", "args"]);
        }

        #[test]
        fn merges_local_args() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml");
            let task = tasks.get("global-test").unwrap();

            assert_eq!(task.command, "global-test");
            assert_eq!(task.args, vec!["--with", "args", "extra", "args"]);
        }
    }

    mod special_options {
        use super::*;

        #[test]
        fn affected_files() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "options/moon.yml");

            let task = tasks.get("affected").unwrap();

            assert_eq!(
                task.options.affected_files,
                Some(TaskOptionAffectedFiles::Enabled(true))
            );

            let task = tasks.get("not-affected").unwrap();

            assert_eq!(
                task.options.affected_files,
                Some(TaskOptionAffectedFiles::Enabled(false))
            );

            let task = tasks.get("affected-args").unwrap();

            assert_eq!(
                task.options.affected_files,
                Some(TaskOptionAffectedFiles::Args)
            );

            let task = tasks.get("affected-env").unwrap();

            assert_eq!(
                task.options.affected_files,
                Some(TaskOptionAffectedFiles::Env)
            );
        }

        #[test]
        fn env_file() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "options/moon.yml");

            let task = tasks.get("env-file").unwrap();

            assert_eq!(
                task.options.env_file,
                Some(InputPath::ProjectFile(".env".into()))
            );

            let task = tasks.get("no-env-file").unwrap();

            assert_eq!(task.options.env_file, None);

            let task = tasks.get("env-file-project").unwrap();

            assert_eq!(
                task.options.env_file,
                Some(InputPath::ProjectFile(".env.test".into()))
            );

            let task = tasks.get("env-file-workspace").unwrap();

            assert_eq!(
                task.options.env_file,
                Some(InputPath::WorkspaceFile(".env.shared".into()))
            );
        }

        #[test]
        fn adds_env_file_as_an_input() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "options/moon.yml");

            let task = tasks.get("env-file").unwrap();

            assert!(task.inputs.contains(&InputPath::ProjectFile(".env".into())));

            let task = tasks.get("no-env-file").unwrap();

            assert!(!task.inputs.contains(&InputPath::ProjectFile(".env".into())));

            let task = tasks.get("env-file-project").unwrap();

            assert!(task
                .inputs
                .contains(&InputPath::ProjectFile(".env.test".into())));

            let task = tasks.get("env-file-workspace").unwrap();

            assert!(task
                .inputs
                .contains(&InputPath::WorkspaceFile(".env.shared".into())));
        }
    }

    mod local_mode {
        use super::*;

        fn is_local(task: &Task) {
            assert!(task.flags.local);
            assert!(!task.options.cache);
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci);
        }

        #[test]
        fn infers_from_task_name() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local-mode/moon.yml");

            is_local(tasks.get("dev").unwrap());
            is_local(tasks.get("start").unwrap());
            is_local(tasks.get("serve").unwrap());
        }

        #[test]
        fn can_override_options() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local-mode/moon.yml");

            let cache = tasks.get("override-cache").unwrap();

            assert!(cache.flags.local);
            assert!(cache.options.cache);

            let style = tasks.get("override-style").unwrap();

            assert!(style.flags.local);
            assert_eq!(style.options.output_style, Some(TaskOutputStyle::Hash));

            let persistent = tasks.get("override-persistent").unwrap();

            assert!(persistent.flags.local);
            assert!(!persistent.options.persistent);

            let ci = tasks.get("override-ci").unwrap();

            assert!(ci.flags.local);
            assert!(ci.options.run_in_ci);
        }

        #[test]
        fn can_override_global_task() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local-mode/moon.yml");

            let build = tasks.get("global-build").unwrap();

            assert!(build.flags.local);

            let run = tasks.get("global-run").unwrap();

            assert!(!run.flags.local);
        }
    }

    mod global_inheritance {
        use super::*;

        fn create_overrides(
            inherited_tasks: ProjectWorkspaceInheritedTasksConfig,
        ) -> ProjectConfig {
            ProjectConfig {
                workspace: ProjectWorkspaceConfig { inherited_tasks },
                ..Default::default()
            }
        }

        #[test]
        fn inherits_all_globals_by_default() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(sandbox.path(), ProjectConfig::default());

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-build", "global-run", "global-test"]
            );
        }

        #[test]
        fn includes_none() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                create_overrides(ProjectWorkspaceInheritedTasksConfig {
                    include: Some(vec![]),
                    ..Default::default()
                }),
            );

            assert!(tasks.is_empty());
        }

        #[test]
        fn includes_by_name() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                create_overrides(ProjectWorkspaceInheritedTasksConfig {
                    include: Some(vec!["global-build".into(), "global-run".into()]),
                    ..Default::default()
                }),
            );

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-build", "global-run"]
            );
        }

        #[test]
        fn excludes_by_name() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                create_overrides(ProjectWorkspaceInheritedTasksConfig {
                    exclude: vec!["global-build".into(), "global-run".into()],
                    ..Default::default()
                }),
            );

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-test"]
            );
        }

        #[test]
        fn excludes_an_included() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                create_overrides(ProjectWorkspaceInheritedTasksConfig {
                    include: Some(vec!["global-build".into(), "global-run".into()]),
                    exclude: vec!["global-build".into()],
                    ..Default::default()
                }),
            );

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-run"]
            );
        }

        #[test]
        fn renames() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                create_overrides(ProjectWorkspaceInheritedTasksConfig {
                    rename: FxHashMap::from_iter([
                        ("global-build".into(), "renamed-build".into()),
                        ("global-test".into(), "renamedTest".into()),
                        ("global-run".into(), "renamed.test".into()),
                    ]),
                    ..Default::default()
                }),
            );

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["renamed-build", "renamed.test", "renamedTest"]
            );
        }
    }

    mod global_implicits {
        use super::*;

        #[test]
        fn no_inputs() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml");
            let task = tasks.get("no-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("project/**/*".into()),
                    InputPath::WorkspaceFile("workspace.json".into()),
                    InputPath::ProjectGlob("**/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert!(!task.flags.empty_inputs);
        }

        #[test]
        fn empty_inputs() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml");
            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("project/**/*".into()),
                    InputPath::WorkspaceFile("workspace.json".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert!(task.flags.empty_inputs);
        }

        #[test]
        fn with_inputs() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml");
            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("project/**/*".into()),
                    InputPath::WorkspaceFile("workspace.json".into()),
                    InputPath::ProjectGlob("local/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert!(!task.flags.empty_inputs);
        }

        #[test]
        fn no_deps() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml");
            let task = tasks.get("no-deps").unwrap();

            assert_eq!(task.deps, vec![Target::parse("app:build").unwrap()]);
        }

        #[test]
        fn empty_deps() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml");
            let task = tasks.get("empty-deps").unwrap();

            assert_eq!(task.deps, vec![Target::parse("app:build").unwrap()]);
        }

        #[test]
        fn with_deps() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml");
            let task = tasks.get("with-deps").unwrap();

            assert_eq!(
                task.deps,
                vec![
                    Target::parse("app:build").unwrap(),
                    Target::parse("^:build").unwrap()
                ]
            );
        }
    }

    mod inputs_scenarios {
        // TODO
    }

    mod merge_strategies {
        // TODO
    }

    mod project_env_vars {
        // TODO
    }
}
