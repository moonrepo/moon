use moon_common::Id;
use moon_config::{
    BunConfig, DenoConfig, InheritedTasksManager, InputPath, NodeConfig, OutputPath, PlatformType,
    ProjectConfig, ProjectWorkspaceConfig, ProjectWorkspaceInheritedTasksConfig, RustConfig,
    TaskArgs, TaskConfig, TaskDependencyConfig, TaskOptionAffectedFiles, TaskOutputStyle, TaskType,
    ToolchainConfig,
};
use moon_target::Target;
use moon_task::Task;
use moon_task_builder::{TasksBuilder, TasksBuilderContext};
use rustc_hash::FxHashMap;
use starbase_sandbox::create_sandbox;
use std::collections::BTreeMap;
use std::path::Path;

async fn build_tasks_with_config(
    root: &Path,
    source: &str,
    local_config: ProjectConfig,
    toolchain_config: ToolchainConfig,
    global_name: Option<&str>,
) -> BTreeMap<Id, Task> {
    let platform = local_config.platform.unwrap_or_default();

    let mut builder = TasksBuilder::new(
        "project",
        source,
        &platform,
        TasksBuilderContext {
            toolchain_config: &toolchain_config,
            workspace_root: root,
        },
    );

    builder.load_local_tasks(&local_config);

    let global_manager =
        InheritedTasksManager::load(root, root.join(global_name.unwrap_or("global"))).unwrap();

    let global_config = global_manager
        .get_inherited_config(
            &platform,
            &local_config.language,
            &local_config.stack,
            &local_config.type_of,
            &local_config.tags,
        )
        .unwrap();

    builder.inherit_global_tasks(
        &global_config.config,
        Some(&local_config.workspace.inherited_tasks),
    );

    builder.build().await.unwrap()
}

async fn build_tasks(root: &Path, config_path: &str) -> BTreeMap<Id, Task> {
    let source = if config_path == "moon.yml" {
        ".".into()
    } else {
        config_path.replace("/moon.yml", "")
    };

    build_tasks_with_config(
        root,
        &source,
        ProjectConfig::create_loader(root.join(&source))
            .unwrap()
            .load()
            .unwrap()
            .config,
        ToolchainConfig::default(),
        None,
    )
    .await
}

async fn build_tasks_with_toolchain(root: &Path, config_path: &str) -> BTreeMap<Id, Task> {
    let source = if config_path == "moon.yml" {
        ".".into()
    } else {
        config_path.replace("/moon.yml", "")
    };

    build_tasks_with_config(
        root,
        &source,
        ProjectConfig::create_loader(root.join(&source))
            .unwrap()
            .load()
            .unwrap()
            .config,
        ToolchainConfig {
            bun: Some(BunConfig::default()),
            deno: Some(DenoConfig::default()),
            node: Some(NodeConfig::default()),
            rust: Some(RustConfig::default()),
            ..ToolchainConfig::default()
        },
        None,
    )
    .await
}

mod tasks_builder {
    use super::*;

    #[tokio::test]
    async fn loads_local_tasks() {
        let sandbox = create_sandbox("builder");
        let tasks = build_tasks(sandbox.path(), "local/moon.yml").await;

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
        assert!(!build.metadata.local_only);

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
        assert!(run.metadata.local_only);

        let test = tasks.get("local-test").unwrap();

        assert_eq!(test.command, "local-test");
        assert_eq!(
            test.inputs,
            vec![
                InputPath::ProjectGlob("**/*".into()),
                InputPath::WorkspaceGlob(".moon/*.yml".into()),
            ]
        );
        assert!(!test.metadata.local_only);
    }

    #[tokio::test]
    async fn inherits_global_tasks() {
        let sandbox = create_sandbox("builder");
        let tasks = build_tasks(sandbox.path(), "local/moon.yml").await;

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
        assert!(!build.metadata.local_only);

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
        assert!(run.metadata.local_only);
    }

    #[tokio::test]
    async fn inherits_global_tasks_from_all_scopes() {
        let sandbox = create_sandbox("builder");
        let tasks = build_tasks(sandbox.path(), "scopes/moon.yml").await;

        assert_eq!(
            tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
            vec![
                "global-build",
                "global-run",
                "global-test",
                "local",
                "node",
                "node-application",
                "tag"
            ]
        );
    }

    mod defaults {
        use super::*;

        #[tokio::test]
        async fn sets_id() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml").await;
            let task = tasks.get("local-build").unwrap();

            assert_eq!(task.id, Id::raw("local-build"));
        }

        #[tokio::test]
        async fn sets_target() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml").await;
            let task = tasks.get("local-build").unwrap();

            assert_eq!(task.target, Target::parse("project:local-build").unwrap());
        }

        #[tokio::test]
        async fn type_test_by_default() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml").await;
            let task = tasks.get("global-test").unwrap();

            assert_eq!(task.type_of, TaskType::Test);
            assert!(task.is_test_type());
            assert!(task.should_run_in_ci());
        }

        #[tokio::test]
        async fn type_run_if_local() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml").await;
            let task = tasks.get("global-run").unwrap();

            assert_eq!(task.type_of, TaskType::Run);
            assert!(task.is_run_type());
            assert!(!task.should_run_in_ci());
        }

        #[tokio::test]
        async fn type_build_if_has_outputs() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml").await;
            let task = tasks.get("global-build").unwrap();

            assert_eq!(task.type_of, TaskType::Build);
            assert!(task.is_build_type());
            assert!(task.should_run_in_ci());
        }
    }

    mod command_args {
        use super::*;

        #[tokio::test]
        async fn command_fallsback_to_noop() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("no-command").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, Vec::<String>::new());
        }

        #[tokio::test]
        async fn command_only() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("command-only").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, Vec::<String>::new());
        }

        #[tokio::test]
        async fn command_string() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("command-string").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test]
        async fn command_list() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("command-list").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test]
        async fn args_string() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("args-string").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test]
        async fn args_list() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("args-list").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test]
        async fn both_string() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("both-string").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test]
        async fn both_list() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("both-list").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test]
        async fn both_list_many() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("both-list-many").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["-qux", "--foo", "bar"]);
        }

        #[tokio::test]
        async fn override_global_command() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("global-build").unwrap();

            assert_eq!(task.command, "override-bin");
            assert_eq!(task.args, vec!["--with", "args"]);
        }

        #[tokio::test]
        async fn merges_local_args() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "commands/moon.yml").await;
            let task = tasks.get("global-test").unwrap();

            assert_eq!(task.command, "global-test");
            assert_eq!(task.args, vec!["--with", "args", "extra", "args"]);
        }

        #[tokio::test]
        async fn handles_args_with_globs() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "args-glob/moon.yml").await;

            let task = tasks.get("no-glob-string").unwrap();

            assert_eq!(task.command, "test");
            assert_eq!(task.args, vec!["./tests"]);
            assert_eq!(
                task.options.shell,
                if cfg!(windows) { Some(true) } else { None }
            );

            let task = tasks.get("with-glob-string").unwrap();

            assert_eq!(task.command, "test");
            assert_eq!(task.args, vec!["./tests/**/*.js"]);
            assert_eq!(task.options.shell, Some(true));

            let task = tasks.get("no-glob-list").unwrap();

            assert_eq!(task.command, "test");
            assert_eq!(task.args, vec!["./tests"]);
            assert_eq!(
                task.options.shell,
                if cfg!(windows) { Some(true) } else { None }
            );

            let task = tasks.get("with-glob-list").unwrap();

            assert_eq!(task.command, "test");
            assert_eq!(task.args, vec!["./tests/**/*.js"]);
            assert_eq!(task.options.shell, Some(true));
        }
    }

    mod detect_platforms {
        use super::*;

        #[tokio::test]
        async fn uses_explicitly_configured() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "platforms/moon.yml").await;

            let task = tasks.get("system").unwrap();

            assert_eq!(task.platform, PlatformType::System);

            let task = tasks.get("bun").unwrap();

            assert_eq!(task.platform, PlatformType::Bun);

            let task = tasks.get("node").unwrap();

            assert_eq!(task.platform, PlatformType::Node);
        }

        #[tokio::test]
        async fn detects_from_command_name() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_toolchain(sandbox.path(), "platforms/moon.yml").await;

            let task = tasks.get("bun-via-cmd").unwrap();

            assert_eq!(task.platform, PlatformType::Bun);

            let task = tasks.get("deno-via-cmd").unwrap();

            assert_eq!(task.platform, PlatformType::Deno);

            let task = tasks.get("node-via-cmd").unwrap();

            assert_eq!(task.platform, PlatformType::Node);

            let task = tasks.get("rust-via-cmd").unwrap();

            assert_eq!(task.platform, PlatformType::Rust);
        }

        #[tokio::test]
        async fn doesnt_detect_from_command_if_not_toolchain_enabled() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "platforms/moon.yml").await;

            let task = tasks.get("bun-via-cmd").unwrap();

            assert_eq!(task.platform, PlatformType::System);

            let task = tasks.get("deno-via-cmd").unwrap();

            assert_eq!(task.platform, PlatformType::System);

            let task = tasks.get("node-via-cmd").unwrap();

            assert_eq!(task.platform, PlatformType::System);

            let task = tasks.get("rust-via-cmd").unwrap();

            assert_eq!(task.platform, PlatformType::System);
        }

        #[tokio::test]
        async fn unknown_fallsback_to_project_platform() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_toolchain(sandbox.path(), "platforms/moon.yml").await;

            let task = tasks.get("unknown").unwrap();

            assert_eq!(task.platform, PlatformType::Rust);

            let task = tasks.get("unknown-implicit").unwrap();

            assert_eq!(task.platform, PlatformType::Rust);
        }

        #[tokio::test]
        async fn applies_to_global_inherited() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_toolchain(sandbox.path(), "platforms/moon.yml").await;

            let task = tasks.get("global-build").unwrap();

            assert_eq!(task.platform, PlatformType::Rust);
        }
    }

    mod special_options {
        use super::*;

        #[tokio::test]
        async fn affected_files() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "options/moon.yml").await;

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

        #[tokio::test]
        async fn env_file() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "options/moon.yml").await;

            let task = tasks.get("env-file").unwrap();

            assert_eq!(
                task.options.env_files,
                Some(vec![InputPath::ProjectFile(".env".into())])
            );

            let task = tasks.get("no-env-file").unwrap();

            assert_eq!(task.options.env_files, None);

            let task = tasks.get("env-file-project").unwrap();

            assert_eq!(
                task.options.env_files,
                Some(vec![InputPath::ProjectFile(".env.test".into())])
            );

            let task = tasks.get("env-file-workspace").unwrap();

            assert_eq!(
                task.options.env_files,
                Some(vec![InputPath::WorkspaceFile(".env.shared".into())])
            );

            let task = tasks.get("env-file-list").unwrap();

            assert_eq!(
                task.options.env_files,
                Some(vec![
                    InputPath::ProjectFile(".env.test".into()),
                    InputPath::WorkspaceFile(".env.shared".into())
                ])
            );
        }

        #[tokio::test]
        async fn adds_env_file_as_an_input() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "options/moon.yml").await;

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

        #[tokio::test]
        async fn interactive() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "options/moon.yml").await;

            let task = tasks.get("interactive").unwrap();

            assert!(!task.options.cache);
            assert!(!task.options.persistent);
            assert!(!task.options.run_in_ci);
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));

            let task = tasks.get("interactive-local").unwrap();

            assert!(!task.options.cache);
            assert!(!task.options.persistent);
            assert!(!task.options.run_in_ci);
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));

            let task = tasks.get("interactive-override").unwrap();

            assert!(!task.options.cache);
            assert!(!task.options.persistent);
            assert!(!task.options.run_in_ci);
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));
        }

        #[tokio::test]
        async fn shell() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "platforms/moon.yml").await;

            // True for system
            assert_eq!(tasks.get("system").unwrap().options.shell, Some(true));

            // None for others (except windows)
            if cfg!(windows) {
                assert_eq!(tasks.get("bun").unwrap().options.shell, Some(true));
                assert_eq!(tasks.get("node").unwrap().options.shell, Some(true));
            } else {
                assert_eq!(tasks.get("bun").unwrap().options.shell, None);
                assert_eq!(tasks.get("node").unwrap().options.shell, None);
            }
        }
    }

    mod default_options {
        use super::*;

        #[tokio::test]
        async fn inherits_from_global() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "options-default/moon.yml").await;

            let task = tasks.get("retry-default").unwrap();

            assert_eq!(task.options.retry_count, 5);
        }

        #[tokio::test]
        async fn can_override_global() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "options-default/moon.yml").await;

            let task = tasks.get("retry-custom").unwrap();

            assert_eq!(task.options.retry_count, 3);
        }
    }

    mod local_mode {
        use super::*;

        fn is_local(task: &Task) {
            assert!(task.metadata.local_only);
            assert!(!task.options.cache);
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci);
        }

        #[tokio::test]
        async fn infers_from_task_name() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local-mode/moon.yml").await;

            is_local(tasks.get("dev").unwrap());
            is_local(tasks.get("start").unwrap());
            is_local(tasks.get("serve").unwrap());
        }

        #[tokio::test]
        async fn can_override_options() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local-mode/moon.yml").await;

            let cache = tasks.get("override-cache").unwrap();

            assert!(cache.metadata.local_only);
            assert!(cache.options.cache);

            let style = tasks.get("override-style").unwrap();

            assert!(style.metadata.local_only);
            assert_eq!(style.options.output_style, Some(TaskOutputStyle::Hash));

            let persistent = tasks.get("override-persistent").unwrap();

            assert!(persistent.metadata.local_only);
            assert!(!persistent.options.persistent);

            let ci = tasks.get("override-ci").unwrap();

            assert!(ci.metadata.local_only);
            assert!(ci.options.run_in_ci);
        }

        #[tokio::test]
        async fn can_override_global_task() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local-mode/moon.yml").await;

            let build = tasks.get("global-build").unwrap();

            assert!(build.metadata.local_only);

            let run = tasks.get("global-run").unwrap();

            assert!(!run.metadata.local_only);
        }
    }

    mod inputs_scenarios {
        use super::*;

        #[tokio::test]
        async fn handles_different_inputs_values() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "inputs/moon.yml").await;

            let task = tasks.get("no-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("**/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert!(!task.metadata.empty_inputs);

            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![InputPath::WorkspaceGlob(".moon/*.yml".into())]
            );
            assert!(task.metadata.empty_inputs);

            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("local/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert!(!task.metadata.empty_inputs);
        }

        #[tokio::test]
        async fn handles_different_inputs_for_root_tasks() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "moon.yml").await;

            let task = tasks.get("no-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![InputPath::WorkspaceGlob(".moon/*.yml".into())]
            );
            assert!(task.metadata.empty_inputs);
            assert!(task.metadata.root_level);

            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![InputPath::WorkspaceGlob(".moon/*.yml".into())]
            );
            assert!(task.metadata.empty_inputs);
            assert!(task.metadata.root_level);

            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("local/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into())
                ]
            );
            assert!(!task.metadata.empty_inputs);
            assert!(task.metadata.root_level);
        }

        #[tokio::test]
        async fn merges_with_global_tasks() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "inputs/moon.yml").await;

            let task = tasks.get("global-build").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("src/**/*".into()),
                    InputPath::WorkspaceFile("workspace-local".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert!(!task.metadata.empty_inputs);

            let task = tasks.get("global-test").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("local.json".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert!(!task.metadata.empty_inputs);

            let task = tasks.get("global-run").unwrap();

            assert_eq!(
                task.inputs,
                vec![InputPath::WorkspaceGlob(".moon/*.yml".into())]
            );
            assert!(task.metadata.empty_inputs);
        }
    }

    mod merge_strategies {
        use super::*;

        #[tokio::test]
        async fn append() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "merge-append/moon.yml").await;

            let task = tasks.get("args").unwrap();

            assert_eq!(task.args, vec!["a", "b", "c", "x", "y", "z"]);

            let task = tasks.get("deps").unwrap();

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("global:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("local:build").unwrap()),
                ]
            );

            let task = tasks.get("env").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "overwrite".into()),
                    ("KEY2".into(), "value2".into()),
                    ("LOCAL".into(), "true".into()),
                ])
            );

            let task = tasks.get("inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("global".into()),
                    InputPath::ProjectFile("local".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );

            let task = tasks.get("outputs").unwrap();

            assert_eq!(
                task.outputs,
                vec![
                    OutputPath::ProjectFile("global".into()),
                    OutputPath::ProjectFile("local".into()),
                ]
            );
        }

        #[tokio::test]
        async fn prepend() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "merge-prepend/moon.yml").await;

            let task = tasks.get("args").unwrap();

            assert_eq!(task.args, vec!["x", "y", "z", "a", "b", "c"]);

            let task = tasks.get("deps").unwrap();

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("local:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("global:build").unwrap()),
                ]
            );

            let task = tasks.get("env").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "value2".into()),
                    ("LOCAL".into(), "true".into()),
                ])
            );

            let task = tasks.get("inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("local".into()),
                    InputPath::ProjectFile("global".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );

            let task = tasks.get("outputs").unwrap();

            assert_eq!(
                task.outputs,
                vec![
                    OutputPath::ProjectFile("local".into()),
                    OutputPath::ProjectFile("global".into()),
                ]
            );
        }

        #[tokio::test]
        async fn replace() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "merge-replace/moon.yml").await;

            let task = tasks.get("args").unwrap();

            assert_eq!(task.args, vec!["x", "y", "z"]);

            let task = tasks.get("deps").unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("local:build").unwrap()
                )]
            );

            let task = tasks.get("env").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "overwrite".into()),
                    ("LOCAL".into(), "true".into()),
                ])
            );

            let task = tasks.get("inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("local".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );

            let task = tasks.get("outputs").unwrap();

            assert_eq!(task.outputs, vec![OutputPath::ProjectFile("local".into())]);
        }

        #[tokio::test]
        async fn replace_empty() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "merge-replace-empty/moon.yml").await;

            let task = tasks.get("args").unwrap();

            assert!(task.args.is_empty());

            let task = tasks.get("deps").unwrap();

            assert!(task.deps.is_empty());

            let task = tasks.get("env").unwrap();

            assert!(task.env.is_empty());

            let task = tasks.get("inputs").unwrap();

            // inherited
            assert_eq!(task.inputs.len(), 2);
            assert!(task.metadata.empty_inputs);

            let task = tasks.get("outputs").unwrap();

            assert!(task.outputs.is_empty());
        }
    }

    mod project_settings {
        use super::*;

        #[tokio::test]
        async fn inherits_project_env_vars() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "local/moon.yml").await;

            let build = tasks.get("local-build").unwrap();

            assert_eq!(
                build.env,
                FxHashMap::from_iter([
                    ("SCOPE".into(), "project".into()),
                    ("KEY".into(), "value".into()),
                ])
            );

            let run = tasks.get("local-run").unwrap();

            assert_eq!(
                run.env,
                FxHashMap::from_iter([
                    ("SCOPE".into(), "project".into()),
                    ("KEY".into(), "value".into()),
                ])
            );

            let test = tasks.get("local-test").unwrap();

            assert_eq!(
                test.env,
                FxHashMap::from_iter([
                    ("SCOPE".into(), "task".into()),
                    ("KEY".into(), "value".into()),
                    ("KEY2".into(), "value2".into()),
                ])
            );
        }
    }

    mod workspace_overrides {
        use super::*;

        fn create_overrides(
            inherited_tasks: ProjectWorkspaceInheritedTasksConfig,
        ) -> ProjectConfig {
            ProjectConfig {
                workspace: ProjectWorkspaceConfig { inherited_tasks },
                ..Default::default()
            }
        }

        #[tokio::test]
        async fn inherits_all_globals_by_default() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                "project",
                ProjectConfig::default(),
                ToolchainConfig::default(),
                None,
            )
            .await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-build", "global-run", "global-test"]
            );
        }

        #[tokio::test]
        async fn includes_none() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                "project",
                create_overrides(ProjectWorkspaceInheritedTasksConfig {
                    include: Some(vec![]),
                    ..Default::default()
                }),
                ToolchainConfig::default(),
                None,
            )
            .await;

            assert!(tasks.is_empty());
        }

        #[tokio::test]
        async fn includes_by_name() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                "project",
                create_overrides(ProjectWorkspaceInheritedTasksConfig {
                    include: Some(vec![Id::raw("global-build"), Id::raw("global-run")]),
                    ..Default::default()
                }),
                ToolchainConfig::default(),
                None,
            )
            .await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-build", "global-run"]
            );
        }

        #[tokio::test]
        async fn excludes_by_name() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                "project",
                create_overrides(ProjectWorkspaceInheritedTasksConfig {
                    exclude: vec![Id::raw("global-build"), Id::raw("global-run")],
                    ..Default::default()
                }),
                ToolchainConfig::default(),
                None,
            )
            .await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-test"]
            );
        }

        #[tokio::test]
        async fn excludes_an_included() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                "project",
                create_overrides(ProjectWorkspaceInheritedTasksConfig {
                    include: Some(vec![Id::raw("global-build"), Id::raw("global-run")]),
                    exclude: vec![Id::raw("global-build")],
                    ..Default::default()
                }),
                ToolchainConfig::default(),
                None,
            )
            .await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-run"]
            );
        }

        #[tokio::test]
        async fn renames() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                "project",
                create_overrides(ProjectWorkspaceInheritedTasksConfig {
                    rename: FxHashMap::from_iter([
                        (Id::raw("global-build"), Id::raw("renamed-build")),
                        (Id::raw("global-test"), Id::raw("renamedTest")),
                        (Id::raw("global-run"), Id::raw("renamed.test")),
                    ]),
                    ..Default::default()
                }),
                ToolchainConfig::default(),
                None,
            )
            .await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["renamed-build", "renamed.test", "renamedTest"]
            );
        }

        #[tokio::test]
        async fn applies_overrides_to_global_task_deps() {
            let sandbox = create_sandbox("builder");

            let mut project_config = create_overrides(ProjectWorkspaceInheritedTasksConfig {
                exclude: vec![Id::raw("test")],
                rename: FxHashMap::from_iter([(Id::raw("build"), Id::raw("compile"))]),
                ..Default::default()
            });

            project_config.tasks.insert(
                Id::raw("build"),
                TaskConfig {
                    command: TaskArgs::String("build-local".into()),
                    ..Default::default()
                },
            );

            project_config.tasks.insert(
                Id::raw("test"),
                TaskConfig {
                    command: TaskArgs::String("test-local".into()),
                    ..Default::default()
                },
            );

            let tasks = build_tasks_with_config(
                sandbox.path(),
                "project",
                project_config,
                ToolchainConfig::default(),
                Some("global-overrides"),
            )
            .await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["build", "compile", "deploy", "test"]
            );

            assert_eq!(
                tasks.get("deploy").unwrap().deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("~:compile").unwrap()
                )]
            );

            assert_eq!(tasks.get("build").unwrap().command, "build-local");
            assert_eq!(tasks.get("compile").unwrap().command, "build-global");
            assert_eq!(tasks.get("test").unwrap().command, "test-local");
        }
    }

    mod global_implicits {
        use super::*;

        #[tokio::test]
        async fn no_inputs() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml").await;
            let task = tasks.get("no-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("**/*".into()),
                    InputPath::ProjectGlob("project/**/*".into()),
                    InputPath::WorkspaceFile("workspace.json".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert!(!task.metadata.empty_inputs);
        }

        #[tokio::test]
        async fn empty_inputs() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml").await;
            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("project/**/*".into()),
                    InputPath::WorkspaceFile("workspace.json".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert!(task.metadata.empty_inputs);
        }

        #[tokio::test]
        async fn with_inputs() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml").await;
            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("local/*".into()),
                    InputPath::ProjectGlob("project/**/*".into()),
                    InputPath::WorkspaceFile("workspace.json".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
            assert!(!task.metadata.empty_inputs);
        }

        #[tokio::test]
        async fn no_deps() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml").await;
            let task = tasks.get("no-deps").unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("app:build").unwrap()
                )]
            );
        }

        #[tokio::test]
        async fn empty_deps() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml").await;
            let task = tasks.get("empty-deps").unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("app:build").unwrap()
                )]
            );
        }

        #[tokio::test]
        async fn with_deps() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "implicits/moon.yml").await;
            let task = tasks.get("with-deps").unwrap();

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("^:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("app:build").unwrap()),
                ]
            );
        }
    }

    mod env_var_merging {
        use super::*;

        #[tokio::test]
        async fn no_env() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "env/moon.yml").await;
            let task = tasks.get("no-env").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("SCOPE".into(), "project".into()),
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "value2".into()),
                ])
            );
        }

        #[tokio::test]
        async fn with_env() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "env/moon.yml").await;
            let task = tasks.get("with-env").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("SCOPE".into(), "task".into()),
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "env-value2".into()),
                    ("EXTRA".into(), "123".into()),
                ])
            );
        }
    }

    mod extending {
        use super::*;

        #[tokio::test]
        #[should_panic(expected = "Task base is extending an unknown task unknown.")]
        async fn errors_for_unknown_extend_task() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "extends-unknown/moon.yml").await;

            tasks.get("base").unwrap();
        }

        #[tokio::test]
        async fn handles_args() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "extends/moon.yml").await;
            let task = tasks.get("extend-args").unwrap();

            assert_eq!(task.command, "lint");
            assert_eq!(task.args, vec!["--fix", "./src"]);
        }

        #[tokio::test]
        async fn handles_inputs() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "extends/moon.yml").await;
            let task = tasks.get("extend-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("src/**/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                ]
            );
        }

        #[tokio::test]
        async fn handles_options() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "extends/moon.yml").await;
            let task = tasks.get("extend-options").unwrap();

            assert!(!task.options.cache);
            assert!(task.options.run_in_ci);
            assert!(task.options.persistent);
            assert_eq!(task.options.retry_count, 3);
        }

        #[tokio::test]
        async fn handles_local() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "extends/moon.yml").await;
            let task = tasks.get("extend-local").unwrap();

            assert!(task.options.cache);
            assert!(task.options.run_in_ci);
            assert!(!task.options.persistent);
        }

        #[tokio::test]
        async fn inherits_and_merges_globals_extend_chain() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "extends/moon.yml").await;
            let task = tasks.get("extender").unwrap();

            assert_eq!(task.command, "global-base");
            assert_eq!(task.args, vec!["-qux", "--foo", "--bar", "-z"]);
            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("global-base".into()),
                    InputPath::ProjectFile("global-extender".into()),
                    InputPath::ProjectFile("local-base".into()),
                    InputPath::ProjectFile("local-extender".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-extends.yml".into()),
                ]
            );

            assert!(task.options.cache);
            assert!(!task.options.run_in_ci);
            assert!(task.options.persistent);
            assert_eq!(task.options.retry_count, 3);
        }

        #[tokio::test]
        async fn can_extend_a_global_from_local() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "extends/moon.yml").await;
            let task = tasks.get("local-extends-global").unwrap();

            assert_eq!(task.command, "global-base");
            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("global-base".into()),
                    InputPath::ProjectFile("local-extender".into()),
                    InputPath::WorkspaceGlob(".moon/*.yml".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-extends.yml".into()),
                ]
            );
        }

        #[tokio::test]
        async fn can_create_extends_chains() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "extends/moon.yml").await;
            let task = tasks.get("extend-args-again").unwrap();

            assert_eq!(task.command, "lint");
            assert_eq!(task.args, vec!["./src", "--fix", "--bail"]);
        }

        #[tokio::test]
        async fn can_interweave_global_and_local_extends() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks_with_config(
                sandbox.path(),
                "extends-interweave",
                ProjectConfig::load_from(sandbox.path(), "extends-interweave").unwrap(),
                ToolchainConfig::default(),
                Some("global-interweave"),
            )
            .await;
            let task = tasks.get("child").unwrap();

            assert_eq!(task.command, "global-parent");
            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("VAR2".into(), "global-child".into()),
                    ("VAR3".into(), "local-child".into()),
                    ("VAR1".into(), "local-parent".into()),
                ])
            )
        }
    }

    mod scripts {
        use super::*;

        #[tokio::test]
        async fn single_command() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "scripts/moon.yml").await;
            let task = tasks.get("single-command").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<String>::new());
            assert_eq!(task.script.as_ref().unwrap(), "foo --bar baz");
        }

        #[tokio::test]
        async fn multi_command() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "scripts/moon.yml").await;
            let task = tasks.get("multi-command").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<String>::new());
            assert_eq!(task.script.as_ref().unwrap(), "foo --bar baz && qux -abc");

            let task = tasks.get("multi-command-semi").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<String>::new());
            assert_eq!(
                task.script.as_ref().unwrap(),
                "foo --bar baz; qux -abc; what"
            );
        }

        #[tokio::test]
        async fn pipe_redirect() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "scripts/moon.yml").await;
            let task = tasks.get("pipe").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<String>::new());
            assert_eq!(task.script.as_ref().unwrap(), "foo | bar | baz");

            let task = tasks.get("redirect").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<String>::new());
            assert_eq!(task.script.as_ref().unwrap(), "foo > bar.txt");
        }

        #[tokio::test]
        async fn replaces_other_command() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "scripts/moon.yml").await;
            let task = tasks.get("with-command").unwrap();

            assert_ne!(task.command, "qux");
        }

        #[tokio::test]
        async fn removes_all_arguments() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "scripts/moon.yml").await;
            let task = tasks.get("with-args").unwrap();

            assert_eq!(task.args, Vec::<String>::new());
        }

        #[tokio::test]
        async fn cannot_disable_shell() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "scripts/moon.yml").await;
            let task = tasks.get("no-shell").unwrap();

            assert_ne!(task.options.shell, Some(true));
        }

        #[tokio::test]
        async fn cannot_change_platform() {
            let sandbox = create_sandbox("builder");
            let tasks = build_tasks(sandbox.path(), "scripts/moon.yml").await;
            let task = tasks.get("custom-platform").unwrap();

            assert_eq!(task.platform, PlatformType::System);
        }
    }
}
