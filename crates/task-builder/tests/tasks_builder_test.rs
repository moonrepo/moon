mod utils;

use moon_common::Id;
use moon_config::*;
use moon_target::Target;
use moon_task::Task;
use rustc_hash::FxHashMap;
use starbase_sandbox::create_sandbox;
use utils::TasksBuilderContainer;

mod tasks_builder {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_local_tasks() {
        let sandbox = create_sandbox("builder");
        let container = TasksBuilderContainer::new(sandbox.path());

        let tasks = container.build_tasks("local").await;
        let build = tasks.get("local-build").unwrap();

        assert_eq!(build.command, "local-build");
        assert_eq!(
            build.inputs,
            vec![
                InputPath::ProjectFile("abc".into()),
                InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
            ]
        );
        assert_eq!(build.outputs, vec![OutputPath::ProjectFile("out".into())]);
        assert!(!build.state.local_only);

        let run = tasks.get("local-run").unwrap();

        assert_eq!(run.command, "local-run");
        assert_eq!(
            run.inputs,
            vec![
                InputPath::ProjectFile("xyz".into()),
                InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
            ]
        );
        assert_eq!(run.outputs, vec![]);
        assert!(run.state.local_only);

        let test = tasks.get("local-test").unwrap();

        assert_eq!(test.command, "local-test");
        assert_eq!(
            test.inputs,
            vec![
                InputPath::ProjectGlob("**/*".into()),
                InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
            ]
        );
        assert!(!test.state.local_only);
    }

    mod inheritance {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_global_tasks() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local").await;
            let build = tasks.get("local-build").unwrap();

            assert_eq!(build.command, "local-build");
            assert_eq!(
                build.inputs,
                vec![
                    InputPath::ProjectFile("abc".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                ]
            );
            assert_eq!(build.outputs, vec![OutputPath::ProjectFile("out".into())]);
            assert!(!build.state.local_only);

            let run = tasks.get("local-run").unwrap();

            assert_eq!(run.command, "local-run");
            assert_eq!(
                run.inputs,
                vec![
                    InputPath::ProjectFile("xyz".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                ]
            );
            assert_eq!(run.outputs, vec![]);
            assert!(run.state.local_only);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_global_tasks_from_all_scopes() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("scopes").await;

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
    }

    mod defaults {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_id() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local").await;
            let task = tasks.get("local-build").unwrap();

            assert_eq!(task.id, Id::raw("local-build"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_target() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local").await;
            let task = tasks.get("local-build").unwrap();

            assert_eq!(task.target, Target::parse("local:local-build").unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn type_test_by_default() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local").await;
            let task = tasks.get("global-test").unwrap();

            assert_eq!(task.type_of, TaskType::Test);
            assert!(task.is_test_type());
            assert!(task.should_run_in_ci());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn type_run_if_local() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local").await;
            let task = tasks.get("global-run").unwrap();

            assert_eq!(task.type_of, TaskType::Run);
            assert!(task.is_run_type());
            assert!(!task.should_run_in_ci());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn type_build_if_has_outputs() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local").await;
            let task = tasks.get("global-build").unwrap();

            assert_eq!(task.type_of, TaskType::Build);
            assert!(task.is_build_type());
            assert!(task.should_run_in_ci());
        }
    }

    mod command_args {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn command_fallsback_to_noop() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("no-command").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, Vec::<String>::new());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn command_only() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("command-only").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, Vec::<String>::new());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn command_string() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("command-string").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn command_list() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("command-list").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn args_string() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("args-string").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn args_list() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("args-list").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn both_string() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("both-string").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn both_list() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("both-list").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["--foo", "bar"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn both_list_many() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("both-list-many").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, vec!["-qux", "--foo", "bar"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn override_global_command() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("global-build").unwrap();

            assert_eq!(task.command, "override-bin");
            assert_eq!(task.args, vec!["--with", "args"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn merges_local_args() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("global-test").unwrap();

            assert_eq!(task.command, "global-test");
            assert_eq!(task.args, vec!["--with", "args", "extra", "args"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_args_with_globs() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("args-glob").await;
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

    mod detect_platform_legacy {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_explicitly_configured() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("platforms").await;

            let task = tasks.get("system").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);

            let task = tasks.get("bun").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("bun")]);

            let task = tasks.get("node").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("node")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_from_command_name() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("platforms").await;

            let task = tasks.get("bun-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("bun")]);

            let task = tasks.get("deno-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("deno")]);

            let task = tasks.get("node-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("node")]);

            let task = tasks.get("rust-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("rust")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_detect_from_command_if_not_toolchain_enabled() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("platforms").await;

            let task = tasks.get("bun-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);

            let task = tasks.get("deno-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);

            let task = tasks.get("node-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);

            let task = tasks.get("rust-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn unknown_fallsback_to_project_platform() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("platforms").await;

            let task = tasks.get("unknown").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("rust")]);

            let task = tasks.get("unknown-implicit").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("rust")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn applies_to_global_inherited() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("platforms").await;

            let task = tasks.get("global-build").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("rust")]);
        }
    }

    mod detect_toolchains {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_explicitly_configured() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("toolchains").await;

            let task = tasks.get("system").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);

            let task = tasks.get("bun").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("bun")]);

            let task = tasks.get("node").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("node")]);

            let task = tasks.get("typescript").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("typescript")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_from_command_name() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("toolchains").await;

            let task = tasks.get("bun-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("bun")]);

            let task = tasks.get("deno-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("deno")]);

            let task = tasks.get("node-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("node")]);

            let task = tasks.get("rust-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("rust")]);

            // let task = tasks.get("typescript-via-cmd").unwrap();

            // assert_eq!(task.toolchains, vec![Id::raw("typescript")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_detect_from_command_if_not_toolchain_enabled() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("toolchains").await;

            let task = tasks.get("bun-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);

            let task = tasks.get("deno-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);

            let task = tasks.get("node-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);

            let task = tasks.get("rust-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);

            let task = tasks.get("typescript-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn unknown_fallsback_to_project_toolchain() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("toolchains").await;

            let task = tasks.get("unknown").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("rust")]);

            let task = tasks.get("unknown-implicit").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("rust")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn applies_to_global_inherited() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("toolchains").await;

            let task = tasks.get("global-build").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("rust")]);
        }
    }

    mod special_options {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn affected_files() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("options").await;

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

        #[tokio::test(flavor = "multi_thread")]
        async fn env_file() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("options").await;

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

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_env_file_as_an_input() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("options").await;

            let task = tasks.get("env-file").unwrap();

            assert!(task.inputs.contains(&InputPath::ProjectFile(".env".into())));

            let task = tasks.get("no-env-file").unwrap();

            assert!(!task.inputs.contains(&InputPath::ProjectFile(".env".into())));

            let task = tasks.get("env-file-project").unwrap();

            assert!(
                task.inputs
                    .contains(&InputPath::ProjectFile(".env.test".into()))
            );

            let task = tasks.get("env-file-workspace").unwrap();

            assert!(
                task.inputs
                    .contains(&InputPath::WorkspaceFile(".env.shared".into()))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn interactive() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("options").await;

            let task = tasks.get("interactive").unwrap();

            // assert!(!task.options.cache);
            // assert!(!task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));

            let task = tasks.get("interactive-local").unwrap();

            // assert!(!task.options.cache);
            // assert!(!task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));

            let task = tasks.get("interactive-override").unwrap();

            // assert!(!task.options.cache);
            // assert!(!task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn shell() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("toolchains").await;

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

        #[tokio::test(flavor = "multi_thread")]
        async fn os() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("options").await;

            let task = tasks.get("os-one").unwrap();

            assert_eq!(task.options.os, Some(vec![TaskOperatingSystem::Windows]));

            let task = tasks.get("os-many").unwrap();

            assert_eq!(
                task.options.os,
                Some(vec![TaskOperatingSystem::Linux, TaskOperatingSystem::Macos])
            );

            let task = tasks.get("os-none").unwrap();

            assert_eq!(task.options.os, Some(vec![]));
        }
    }

    mod default_options {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_from_global() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("options-default").await;

            let task = tasks.get("retry-default").unwrap();

            assert_eq!(task.options.retry_count, 5);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_override_global() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("options-default").await;

            let task = tasks.get("retry-custom").unwrap();

            assert_eq!(task.options.retry_count, 3);
        }
    }

    mod local_mode {
        use super::*;

        fn is_local(task: &Task) {
            assert!(task.state.local_only);
            assert!(!task.options.cache);
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn infers_from_task_name() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local-mode").await;

            is_local(tasks.get("dev").unwrap());
            is_local(tasks.get("start").unwrap());
            is_local(tasks.get("serve").unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_override_options() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local-mode").await;

            let cache = tasks.get("override-cache").unwrap();

            assert!(cache.state.local_only);
            assert!(cache.options.cache);

            let style = tasks.get("override-style").unwrap();

            assert!(style.state.local_only);
            assert_eq!(style.options.output_style, Some(TaskOutputStyle::Hash));

            let persistent = tasks.get("override-persistent").unwrap();

            assert!(persistent.state.local_only);
            assert!(!persistent.options.persistent);

            let ci = tasks.get("override-ci").unwrap();

            assert!(ci.state.local_only);
            assert!(ci.options.run_in_ci.is_enabled());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_override_global_task() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local-mode").await;

            let build = tasks.get("global-build").unwrap();

            assert!(build.state.local_only);

            let run = tasks.get("global-run").unwrap();

            assert!(!run.state.local_only);
        }
    }

    mod presets {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn server() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("presets").await;

            let task = tasks.get("server").unwrap();

            assert_eq!(task.preset, Some(TaskPreset::Server));
            assert!(!task.options.cache);
            assert!(!task.options.interactive);
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));

            // Custom overrides
            let task = tasks.get("server-custom").unwrap();

            assert_eq!(task.preset, Some(TaskPreset::Server));
            assert!(task.options.cache);
            assert!(!task.options.interactive);
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));

            // Extends a task with no preset, so inherits further
            let task = tasks.get("server-extends").unwrap();

            assert_eq!(task.preset, Some(TaskPreset::Server));
            assert!(task.options.cache);
            assert!(!task.options.interactive);
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn watcher() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("presets").await;

            let task = tasks.get("watcher").unwrap();

            assert_eq!(task.preset, Some(TaskPreset::Watcher));
            assert!(!task.options.cache);
            assert!(task.options.interactive);
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));

            // Custom overrides
            let task = tasks.get("watcher-custom").unwrap();

            assert_eq!(task.preset, Some(TaskPreset::Watcher));
            assert!(!task.options.cache);
            assert!(!task.options.interactive);
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));
        }
    }

    mod inputs_scenarios {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_different_inputs_values() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("inputs").await;

            let task = tasks.get("no-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("**/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                ]
            );
            assert!(!task.state.empty_inputs);

            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into())]
            );
            assert!(task.state.empty_inputs);

            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("local/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                ]
            );
            assert!(!task.state.empty_inputs);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_different_inputs_for_root_tasks() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("root").await;

            let task = tasks.get("no-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into())]
            );
            assert!(task.state.empty_inputs);
            assert!(task.state.root_level);

            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into())]
            );
            assert!(task.state.empty_inputs);
            assert!(task.state.root_level);

            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("local/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into())
                ]
            );
            assert!(!task.state.empty_inputs);
            assert!(task.state.root_level);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_different_inputs_for_root_tasks_when_a_polyrepo() {
            let sandbox = create_sandbox("builder-poly");
            let container = TasksBuilderContainer::new(sandbox.path()).with_polyrepo();

            let tasks = container.build_tasks("root").await;

            let task = tasks.get("no-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("**/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into())
                ]
            );
            assert!(!task.state.empty_inputs);
            assert!(task.state.root_level);

            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into())]
            );
            assert!(task.state.empty_inputs);
            assert!(task.state.root_level);

            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("local/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into())
                ]
            );
            assert!(!task.state.empty_inputs);
            assert!(task.state.root_level);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn merges_with_global_tasks() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("inputs").await;

            let task = tasks.get("global-build").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("src/**/*".into()),
                    InputPath::WorkspaceFile("workspace-local".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                ]
            );
            assert!(!task.state.empty_inputs);

            let task = tasks.get("global-test").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("local.json".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                ]
            );
            assert!(!task.state.empty_inputs);

            let task = tasks.get("global-run").unwrap();

            assert_eq!(
                task.inputs,
                vec![InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into())]
            );
            assert!(task.state.empty_inputs);
        }
    }

    mod merge_strategies {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn append() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-append").await;

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
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
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

        #[tokio::test(flavor = "multi_thread")]
        async fn append_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-append").await;

            let task = tasks.get("all").unwrap();

            assert_eq!(task.args, vec!["a", "b", "c", "x", "y", "z"]);

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("global:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("local:build").unwrap()),
                ]
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "overwrite".into()),
                    ("KEY2".into(), "value2".into()),
                    ("LOCAL".into(), "true".into()),
                ])
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("global".into()),
                    InputPath::ProjectFile("local".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.outputs,
                vec![
                    OutputPath::ProjectFile("global".into()),
                    OutputPath::ProjectFile("local".into()),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prepend() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-prepend").await;

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
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
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

        #[tokio::test(flavor = "multi_thread")]
        async fn prepend_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-prepend").await;

            let task = tasks.get("all").unwrap();

            assert_eq!(task.args, vec!["x", "y", "z", "a", "b", "c"]);

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("local:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("global:build").unwrap()),
                ]
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "value2".into()),
                    ("LOCAL".into(), "true".into()),
                ])
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("local".into()),
                    InputPath::ProjectFile("global".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.outputs,
                vec![
                    OutputPath::ProjectFile("local".into()),
                    OutputPath::ProjectFile("global".into()),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn replace() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-replace").await;

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
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );

            let task = tasks.get("outputs").unwrap();

            assert_eq!(task.outputs, vec![OutputPath::ProjectFile("local".into())]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn replace_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-replace").await;

            let task = tasks.get("all").unwrap();

            assert_eq!(task.args, vec!["x", "y", "z"]);

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("local:build").unwrap()
                )]
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "overwrite".into()),
                    ("LOCAL".into(), "true".into()),
                ])
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("local".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(task.outputs, vec![OutputPath::ProjectFile("local".into())]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn replace_empty() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-replace-empty").await;

            let task = tasks.get("args").unwrap();

            assert!(task.args.is_empty());

            let task = tasks.get("deps").unwrap();

            assert!(task.deps.is_empty());

            let task = tasks.get("env").unwrap();

            assert!(task.env.is_empty());

            let task = tasks.get("inputs").unwrap();

            // inherited
            assert_eq!(task.inputs.len(), 2);
            assert!(task.state.empty_inputs);

            let task = tasks.get("outputs").unwrap();

            assert!(task.outputs.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn replace_empty_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-replace-empty").await;

            let task = tasks.get("all").unwrap();

            assert!(task.args.is_empty());

            let task = tasks.get("all").unwrap();

            assert!(task.deps.is_empty());

            let task = tasks.get("all").unwrap();

            assert!(task.env.is_empty());

            let task = tasks.get("all").unwrap();

            // inherited
            assert_eq!(task.inputs.len(), 2);
            assert!(task.state.empty_inputs);

            let task = tasks.get("all").unwrap();

            assert!(task.outputs.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_replace_undefined() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-replace-undefined").await;

            let task = tasks.get("args").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, vec!["a", "b", "c"]);

            let task = tasks.get("deps").unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("global:build").unwrap()
                )]
            );

            let task = tasks.get("env").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "value2".into()),
                ])
            );

            let task = tasks.get("inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("global".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );
            assert!(!task.state.empty_inputs);

            let task = tasks.get("outputs").unwrap();

            assert_eq!(task.outputs, vec![OutputPath::ProjectFile("global".into())]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_replace_undefined_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-replace-undefined").await;

            let task = tasks.get("all").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, vec!["a", "b", "c"]);

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("global:build").unwrap()
                )]
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "value2".into()),
                ])
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("global".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );
            assert!(!task.state.empty_inputs);

            let task = tasks.get("all").unwrap();

            assert_eq!(task.outputs, vec![OutputPath::ProjectFile("global".into())]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn preserve() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-preserve").await;

            let task = tasks.get("args").unwrap();

            assert_eq!(task.args, vec!["a", "b", "c"]);

            let task = tasks.get("deps").unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("global:build").unwrap()
                )]
            );

            let task = tasks.get("env").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "value2".into()),
                ])
            );

            let task = tasks.get("inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("global".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );

            let task = tasks.get("outputs").unwrap();

            assert_eq!(task.outputs, vec![OutputPath::ProjectFile("global".into())]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn preserve_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-preserve").await;

            let task = tasks.get("all").unwrap();

            assert_eq!(task.args, vec!["a", "b", "c"]);

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("global:build").unwrap()
                )]
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "value2".into()),
                ])
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("global".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-merge.yml".into()),
                ]
            );

            let task = tasks.get("all").unwrap();

            assert_eq!(task.outputs, vec![OutputPath::ProjectFile("global".into())]);
        }
    }

    mod project_settings {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_project_env_vars() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local").await;

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

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_all_globals_by_default() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("no-tasks").await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-build", "global-run", "global-test"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_none() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("override-none").await;

            assert!(tasks.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_by_name() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("override-include").await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-build", "global-run"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn excludes_by_name() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("override-exclude").await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-test"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn excludes_an_included() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("override-overlap").await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["global-run"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn renames() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("override-rename").await;

            assert_eq!(
                tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                vec!["renamed-build", "renamed.test", "renamedTest"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn applies_overrides_to_global_task_deps() {
            let sandbox = create_sandbox("builder");
            let container =
                TasksBuilderContainer::new(sandbox.path()).with_global_tasks("global-overrides");

            let tasks = container.build_tasks("override-global").await;

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

        #[tokio::test(flavor = "multi_thread")]
        async fn no_inputs() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("implicits").await;
            let task = tasks.get("no-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("**/*".into()),
                    InputPath::ProjectGlob("project/**/*".into()),
                    InputPath::WorkspaceFile("workspace.json".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                ]
            );
            assert!(!task.state.empty_inputs);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn empty_inputs() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("implicits").await;
            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("project/**/*".into()),
                    InputPath::WorkspaceFile("workspace.json".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                ]
            );
            assert!(task.state.empty_inputs);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn with_inputs() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("implicits").await;
            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("local/*".into()),
                    InputPath::ProjectGlob("project/**/*".into()),
                    InputPath::WorkspaceFile("workspace.json".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                ]
            );
            assert!(!task.state.empty_inputs);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_deps() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("implicits").await;
            let task = tasks.get("no-deps").unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("app:build").unwrap()
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn empty_deps() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("implicits").await;
            let task = tasks.get("empty-deps").unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("app:build").unwrap()
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn with_deps() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("implicits").await;
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

        #[tokio::test(flavor = "multi_thread")]
        async fn no_env() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("env").await;
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

        #[tokio::test(flavor = "multi_thread")]
        async fn with_env() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("env").await;
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

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Task base is extending an unknown task unknown.")]
        async fn errors_for_unknown_extend_task() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("extends-unknown").await;

            tasks.get("base").unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_args() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("extends").await;
            let task = tasks.get("extend-args").unwrap();

            assert_eq!(task.command, "lint");
            assert_eq!(task.args, vec!["--fix", "./src"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_inputs() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("extends").await;
            let task = tasks.get("extend-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectGlob("src/**/*".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_options() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("extends").await;
            let task = tasks.get("extend-options").unwrap();

            assert!(!task.options.cache);
            assert!(task.options.run_in_ci.is_enabled());
            assert!(task.options.persistent);
            assert_eq!(task.options.retry_count, 3);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_local() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("extends").await;
            let task = tasks.get("extend-local").unwrap();

            assert!(task.options.cache);
            assert!(task.options.run_in_ci.is_enabled());
            assert!(!task.options.persistent);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_and_merges_globals_extend_chain() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("extends").await;
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
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-extends.yml".into()),
                ]
            );

            assert!(task.options.cache);
            assert!(!task.options.run_in_ci.is_enabled());
            assert!(task.options.persistent);
            assert_eq!(task.options.retry_count, 3);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_extend_a_global_from_local() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("extends").await;
            let task = tasks.get("local-extends-global").unwrap();

            assert_eq!(task.command, "global-base");
            assert_eq!(
                task.inputs,
                vec![
                    InputPath::ProjectFile("global-base".into()),
                    InputPath::ProjectFile("local-extender".into()),
                    InputPath::WorkspaceGlob(".moon/*.{pkl,yml}".into()),
                    InputPath::WorkspaceFile("global/tasks/tag-extends.yml".into()),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_create_extends_chains() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("extends").await;
            let task = tasks.get("extend-args-again").unwrap();

            assert_eq!(task.command, "lint");
            assert_eq!(task.args, vec!["./src", "--fix", "--bail"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_interweave_global_and_local_extends() {
            let sandbox = create_sandbox("builder");
            let container =
                TasksBuilderContainer::new(sandbox.path()).with_global_tasks("global-interweave");

            let tasks = container.build_tasks("extends-interweave").await;
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

        #[tokio::test(flavor = "multi_thread")]
        async fn single_command() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("scripts").await;
            let task = tasks.get("single-command").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<String>::new());
            assert_eq!(task.script.as_ref().unwrap(), "foo --bar baz");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn multi_command() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("scripts").await;
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

        #[tokio::test(flavor = "multi_thread")]
        async fn pipe_redirect() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("scripts").await;
            let task = tasks.get("pipe").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<String>::new());
            assert_eq!(task.script.as_ref().unwrap(), "foo | bar | baz");

            let task = tasks.get("redirect").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<String>::new());
            assert_eq!(task.script.as_ref().unwrap(), "foo > bar.txt");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn replaces_other_command() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("scripts").await;
            let task = tasks.get("with-command").unwrap();

            assert_ne!(task.command, "qux");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_all_arguments() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("scripts").await;
            let task = tasks.get("with-args").unwrap();

            assert_eq!(task.args, Vec::<String>::new());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn cannot_disable_shell() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("scripts").await;
            let task = tasks.get("no-shell").unwrap();

            assert_ne!(task.options.shell, Some(true));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn cannot_change_platform_legacy() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("scripts").await;
            let task = tasks.get("custom-platform").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn cannot_change_toolchain() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("scripts").await;
            let task = tasks.get("custom-toolchain").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("system")]);
        }
    }

    mod os_targeting {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_linux() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("options-os").await;
            let task = tasks.get("os-linux").unwrap();

            assert_eq!(task.options.os, Some(vec![TaskOperatingSystem::Linux]));

            if cfg!(target_os = "linux") {
                assert_eq!(task.command, "execute");
                assert_eq!(task.args, Vec::<String>::new());
                assert_eq!(task.script, Some("execute --nix".to_owned()));
            } else {
                assert_eq!(task.command, "noop");
                assert_eq!(task.args, Vec::<String>::new());
                assert_eq!(task.script, None);
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_macos() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("options-os").await;
            let task = tasks.get("os-macos").unwrap();

            assert_eq!(task.options.os, Some(vec![TaskOperatingSystem::Macos]));

            if cfg!(target_os = "macos") {
                assert_eq!(task.command, "execute");
                assert_eq!(task.args, ["--mac"]);
                assert_eq!(task.script, None);
            } else {
                assert_eq!(task.command, "noop");
                assert_eq!(task.args, Vec::<String>::new());
                assert_eq!(task.script, None);
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_windows() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("options-os").await;
            let task = tasks.get("os-windows").unwrap();

            assert_eq!(task.options.os, Some(vec![TaskOperatingSystem::Windows]));

            if cfg!(target_os = "windows") {
                assert_eq!(task.command, "execute");
                assert_eq!(task.args, ["--win"]);
                assert_eq!(task.script, None);
            } else {
                assert_eq!(task.command, "noop");
                assert_eq!(task.args, Vec::<String>::new());
                assert_eq!(task.script, None);
            }
        }
    }
}
