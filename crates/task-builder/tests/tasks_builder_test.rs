mod utils;

use moon_common::Id;
use moon_config::{test_utils::*, *};
use moon_target::Target;
use moon_task::{TaskArg, TaskOptionAffectedFiles};
use starbase_sandbox::create_sandbox;
use std::fs;
use std::path::PathBuf;
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
                Input::File(stub_file_input("abc")),
                Input::Glob(stub_glob_input(
                    "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                )),
            ]
        );
        assert_eq!(build.outputs, vec![Output::File(stub_file_output("out"))]);

        let run = tasks.get("local-run").unwrap();

        assert_eq!(run.command, "local-run");
        assert_eq!(
            run.inputs,
            vec![
                Input::File(stub_file_input("xyz")),
                Input::Glob(stub_glob_input(
                    "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                )),
            ]
        );
        assert_eq!(run.outputs, vec![]);

        let test = tasks.get("local-test").unwrap();

        assert_eq!(test.command, "local-test");
        assert_eq!(
            test.inputs,
            vec![
                Input::Glob(stub_glob_input("**/*")),
                Input::Glob(stub_glob_input(
                    "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                )),
            ]
        );
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
                    Input::File(stub_file_input("abc")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                ]
            );
            assert_eq!(build.outputs, vec![Output::File(stub_file_output("out"))]);

            let run = tasks.get("local-run").unwrap();

            assert_eq!(run.command, "local-run");
            assert_eq!(
                run.inputs,
                vec![
                    Input::File(stub_file_input("xyz")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                ]
            );
            assert_eq!(run.outputs, vec![]);
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

        #[tokio::test(flavor = "multi_thread")]
        async fn deep_merges_global_tasks() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let mut tasks = container.build_tasks("inheritance").await;
            let task = tasks.remove("build").unwrap();

            assert_eq!(task.command, "build");
            assert_eq!(task.args, ["--one", "--two", "--three", "value"]);
            assert_eq!(task.preset.unwrap(), TaskPreset::Server);
            assert_eq!(task.options.cache_lifetime.unwrap(), "7 days");
            assert_eq!(task.options.mutex.unwrap(), "lock-overwrite");
            assert!(task.options.interactive);
            // Off because of interactive
            assert_eq!(task.options.run_in_ci, TaskOptionRunInCI::Enabled(false));
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
            assert!(task.should_run(true));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn type_run_if_local() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local").await;
            let task = tasks.get("global-run").unwrap();

            assert_eq!(task.type_of, TaskType::Run);
            assert!(task.is_run_type());
            assert!(!task.should_run(true));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn type_build_if_has_outputs() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("local").await;
            let task = tasks.get("global-build").unwrap();

            assert_eq!(task.type_of, TaskType::Build);
            assert!(task.is_build_type());
            assert!(task.should_run(true));
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
            assert_eq!(task.args, Vec::<TaskArg>::new());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn command_only() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("commands").await;
            let task = tasks.get("command-only").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(task.args, Vec::<TaskArg>::new());
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
            assert_eq!(task.options.shell, Some(true));

            let task = tasks.get("with-glob-string").unwrap();

            assert_eq!(task.command, "test");
            assert_eq!(task.args, vec!["./tests/**/*.js"]);
            assert_eq!(task.options.shell, Some(true));

            let task = tasks.get("no-glob-list").unwrap();

            assert_eq!(task.command, "test");
            assert_eq!(task.args, vec!["./tests"]);
            assert_eq!(task.options.shell, Some(true));

            let task = tasks.get("with-glob-list").unwrap();

            assert_eq!(task.command, "test");
            assert_eq!(task.args, vec!["./tests/**/*.js"]);
            assert_eq!(task.options.shell, Some(true));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_env_substitution() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("env-substitute").await;
            let task = tasks.get("command-no-env").unwrap();

            assert_eq!(task.command, "./file.sh");

            let task = tasks.get("command-with-env").unwrap();

            assert_eq!(task.command, "./${DIR}/file.sh");

            let task = tasks.get("args-no-env").unwrap();

            assert_eq!(task.args, vec!["arg"]);

            let task = tasks.get("args-with-env").unwrap();

            assert_eq!(task.args, vec!["arg", "$ARG"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_auto_shell() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("auto-shell").await;
            let task = tasks.get("with-globs").unwrap();

            assert_eq!(task.options.shell, Some(true));

            let task = tasks.get("with-env").unwrap();

            assert_eq!(task.options.shell, Some(true));

            let task = tasks.get("with-globs-off").unwrap();

            assert_eq!(task.options.shell, Some(false));

            let task = tasks.get("with-env-off").unwrap();

            assert_eq!(task.options.shell, Some(false));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_token_funcs() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("tokens").await;
            let task = tasks.get("funcs-string").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(
                task.args,
                vec![
                    TaskArg::new_unquoted("@group(storybook)"),
                    TaskArg::new_quoted("@root(sources)", "\"@root(sources)\""),
                    TaskArg::new_unquoted("@in(0)"),
                    TaskArg::new_unquoted("@out(0)"),
                    TaskArg::new_unquoted("@meta(title)"),
                    TaskArg::new_unquoted("@meta(index)"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_token_vars() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("tokens").await;
            let task = tasks.get("vars-string").unwrap();

            assert_eq!(task.command, "bin");
            assert_eq!(
                task.args,
                vec![
                    TaskArg::new_unquoted("arg"),
                    TaskArg::new_quoted("$workspaceRoot", "\"$workspaceRoot\""),
                    TaskArg::new_unquoted("$os"),
                    TaskArg::new_quoted("$projectTitle", "'$projectTitle'"),
                    TaskArg::new_unquoted("$projectRoot/in/path.txt"),
                    TaskArg::new_unquoted("./in/$target/path.txt"),
                ]
            );
        }
    }

    mod command_syntax {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn simple() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("syntax").await;
            let task = tasks.get("simple").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, ["-a", "--bar", "baz", "qux"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn passthrough() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("syntax").await;
            let task = tasks.get("passthrough").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, ["--", "bar", "-b"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn preserves_quotes() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("syntax").await;
            let task = tasks.get("quotes").unwrap();

            assert_eq!(task.command, "echo");
            assert_eq!(
                task.args,
                [
                    TaskArg::new_unquoted("noquotes"),
                    TaskArg::new_quoted("single quote", "'single quote'"),
                    TaskArg::new_quoted("double quote", "\"double quote\""),
                    TaskArg::new_quoted("special quote", "$\"special quote\""),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn preserves_quotes_in_exe() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("syntax").await;
            let task = tasks.get("quotes-exe").unwrap();

            assert_eq!(
                task.command,
                TaskArg::new_quoted(
                    "./some/file path/with/spaces.sh",
                    "\"./some/file path/with/spaces.sh\""
                )
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn allows_expansion() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("syntax").await;
            let task = tasks.get("expansion").unwrap();

            assert_eq!(task.command, "echo");
            assert_eq!(
                task.args,
                [
                    TaskArg::new_unquoted("$(( 1+1 ))"),
                    TaskArg::new_unquoted("file/*.txt"),
                    TaskArg::new_unquoted("${foo:bar}"),
                ]
            );
            assert_eq!(task.options.shell, Some(true));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn allows_substitution() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("syntax").await;
            let task = tasks.get("substitution").unwrap();

            assert_eq!(task.command, "echo");
            assert_eq!(
                task.args,
                [
                    TaskArg::new_unquoted("$(do something)"),
                    TaskArg::new_unquoted("<(echo bar)"),
                ]
            );
            assert_eq!(task.options.shell, Some(true));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extracts_env_assignments() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("syntax").await;
            let task = tasks.get("env").unwrap();

            assert_eq!(task.command, "exit");
            assert_eq!(task.args, ["0"]);
            assert_eq!(
                task.env,
                EnvMap::from_iter([
                    ("FOO".into(), Some("abc".into())),
                    ("BAR".into(), Some("123".into())),
                    ("BAZ".into(), Some("quoted value".into())),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extracts_env_assignments_in_separate_statements() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("syntax").await;
            let task = tasks.get("env-multi").unwrap();

            assert_eq!(task.command, "exit");
            assert_eq!(task.args, ["0"]);
            assert_eq!(
                task.env,
                EnvMap::from_iter([
                    ("FOO".into(), Some("abc".into())),
                    ("BAR".into(), Some("123".into())),
                    ("BAZ".into(), Some("quoted value".into())),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_env_assignments_for_non_values() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("syntax").await;
            let task = tasks.get("env-ignore").unwrap();

            assert_eq!(task.command, "exit");
            assert_eq!(task.args, ["0"]);
            assert_eq!(task.env, EnvMap::default());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_env_assignments_when_inside_command() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("syntax").await;
            let task = tasks.get("env-inside").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, ["--env", "FOO=abc", "arg"]);
            assert_eq!(task.env, EnvMap::default());
        }

        // We can't place these invalid commands in the fixture,
        // because they would trigger a failure upon creating
        // the test container and break other tests!
        fn update_command(path: PathBuf, command: &str) {
            let content = fs::read_to_string(&path).unwrap();

            fs::write(path, content.replace("{{ command }}", command)).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unable to build task")]
        async fn errors_for_pipes() {
            let sandbox = create_sandbox("builder");

            update_command(
                sandbox.path().join("syntax-error/moon.yml"),
                "echo foo | grep f",
            );

            TasksBuilderContainer::new(sandbox.path())
                .build_tasks("syntax-error")
                .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unable to build task")]
        async fn errors_for_redirects() {
            let sandbox = create_sandbox("builder");

            update_command(
                sandbox.path().join("syntax-error/moon.yml"),
                "echo foo > file.txt",
            );

            TasksBuilderContainer::new(sandbox.path())
                .build_tasks("syntax-error")
                .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unable to build task")]
        async fn errors_for_and() {
            let sandbox = create_sandbox("builder");

            update_command(
                sandbox.path().join("syntax-error/moon.yml"),
                "echo foo && echo bar",
            );

            TasksBuilderContainer::new(sandbox.path())
                .build_tasks("syntax-error")
                .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unable to build task")]
        async fn errors_for_or() {
            let sandbox = create_sandbox("builder");

            update_command(
                sandbox.path().join("syntax-error/moon.yml"),
                "echo foo || echo bar",
            );

            TasksBuilderContainer::new(sandbox.path())
                .build_tasks("syntax-error")
                .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unable to build task")]
        async fn errors_for_shell_syntax() {
            let sandbox = create_sandbox("builder");

            update_command(
                sandbox.path().join("syntax-error/moon.yml"),
                "if [[ true ]]; echo foo; else; echo bar; fi",
            );

            TasksBuilderContainer::new(sandbox.path())
                .build_tasks("syntax-error")
                .await;
        }
    }

    mod inputs {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn expands_project_all() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("inputs-project").await;
            let task = tasks.get("all-deps").unwrap();

            assert_eq!(
                task.inputs,
                [
                    Input::Project(ProjectInput {
                        project: "dep-b".into(),
                        filter: vec![],
                        group: Some(Id::raw("sources")),
                    }),
                    Input::Project(ProjectInput {
                        project: "dep-a".into(),
                        filter: vec![],
                        group: Some(Id::raw("sources")),
                    }),
                    Input::Project(ProjectInput {
                        project: "dep-c".into(),
                        filter: vec![],
                        group: Some(Id::raw("sources")),
                    }),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_single_project() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("inputs-project").await;
            let task = tasks.get("only-a").unwrap();

            assert_eq!(
                task.inputs,
                [
                    Input::Project(ProjectInput {
                        project: "dep-a".into(),
                        filter: vec!["src/**/*".into()],
                        group: None,
                    }),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Invalid project input")]
        async fn errors_if_referencing_a_non_dep_project() {
            let sandbox = create_sandbox("builder");
            sandbox.create_file(
                "inputs-project-error/moon.yml",
                r#"
dependsOn: ['dep-a']

tasks:
  will-error:
    inputs:
      - project: 'dep-b'
"#,
            );

            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("inputs-project-error").await;
            let task = tasks.get("only-a").unwrap();

            assert_eq!(
                task.inputs,
                [
                    Input::Project(ProjectInput {
                        project: "dep-a".into(),
                        filter: vec!["src/**/*".into()],
                        group: None,
                    }),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                ]
            );
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

            assert_eq!(task.toolchains, vec![Id::raw("npm"), Id::raw("node")]);

            let task = tasks.get("typescript").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("typescript")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_from_command_name() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path()).with_all_toolchains();

            let tasks = container.build_tasks("toolchains").await;

            let task = tasks.get("bun-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("bun"), Id::raw("javascript")]);

            let task = tasks.get("deno-via-cmd").unwrap();

            assert_eq!(
                task.toolchains,
                vec![Id::raw("deno"), Id::raw("javascript")]
            );

            let task = tasks.get("node-via-cmd").unwrap();

            assert_eq!(
                task.toolchains,
                vec![Id::raw("javascript"), Id::raw("npm"), Id::raw("node")]
            );

            let task = tasks.get("rust-via-cmd").unwrap();

            assert_eq!(task.toolchains, vec![Id::raw("rust")]);

            // TODO: temp disabled in the typescript plugin
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

            assert_eq!(task.toolchains, vec![Id::raw("system")]);

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
                Some(TaskOptionAffectedFiles {
                    pass: TaskOptionAffectedFilesPattern::Enabled(true),
                    ..Default::default()
                })
            );

            let task = tasks.get("not-affected").unwrap();

            assert_eq!(
                task.options.affected_files,
                Some(TaskOptionAffectedFiles {
                    pass: TaskOptionAffectedFilesPattern::Enabled(false),
                    ..Default::default()
                })
            );

            let task = tasks.get("affected-args").unwrap();

            assert_eq!(
                task.options.affected_files,
                Some(TaskOptionAffectedFiles {
                    pass: TaskOptionAffectedFilesPattern::Args,
                    ..Default::default()
                })
            );

            let task = tasks.get("affected-env").unwrap();

            assert_eq!(
                task.options.affected_files,
                Some(TaskOptionAffectedFiles {
                    pass: TaskOptionAffectedFilesPattern::Env,
                    ..Default::default()
                })
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
                Some(vec![
                    Input::File(stub_file_input("/.env")),
                    Input::File(stub_file_input("/.env.local")),
                    Input::File(stub_file_input(".env")),
                    Input::File(stub_file_input(".env.local")),
                    Input::File(stub_file_input(".env.env-file")),
                    Input::File(stub_file_input(".env.env-file.local")),
                ])
            );

            let task = tasks.get("no-env-file").unwrap();

            assert_eq!(task.options.env_files, None);

            let task = tasks.get("env-file-project").unwrap();

            assert_eq!(
                task.options.env_files,
                Some(vec![Input::File(stub_file_input(".env.test"))])
            );

            let task = tasks.get("env-file-workspace").unwrap();

            assert_eq!(
                task.options.env_files,
                Some(vec![Input::File(stub_file_input("/.env.shared"))])
            );

            let task = tasks.get("env-file-list").unwrap();

            assert_eq!(
                task.options.env_files,
                Some(vec![
                    Input::File(stub_file_input(".env.test")),
                    Input::File(stub_file_input("/.env.shared"))
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_env_file_as_an_input() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("options").await;

            let task = tasks.get("env-file").unwrap();

            assert!(task.inputs.contains(&Input::File(stub_file_input(".env"))));

            let task = tasks.get("no-env-file").unwrap();

            assert!(!task.inputs.contains(&Input::File(stub_file_input(".env"))));

            let task = tasks.get("env-file-project").unwrap();

            assert!(
                task.inputs
                    .contains(&Input::File(stub_file_input(".env.test")))
            );

            let task = tasks.get("env-file-workspace").unwrap();

            assert!(
                task.inputs
                    .contains(&Input::File(stub_file_input("/.env.shared")))
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

            assert_eq!(tasks.get("system").unwrap().options.shell, Some(true));
            assert_eq!(tasks.get("bun").unwrap().options.shell, Some(true));
            assert_eq!(tasks.get("node").unwrap().options.shell, Some(true));
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

    mod presets {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn server() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("presets").await;

            let task = tasks.get("server").unwrap();

            assert_eq!(task.preset, Some(TaskPreset::Server));
            assert_eq!(task.options.cache, TaskOptionCache::Enabled(false));
            assert!(!task.options.interactive);
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));

            // Custom overrides
            let task = tasks.get("server-custom").unwrap();

            assert_eq!(task.preset, Some(TaskPreset::Server));
            assert_eq!(task.options.cache, TaskOptionCache::Enabled(true));
            assert!(!task.options.interactive);
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));

            // Extends a task with no preset, so inherits further
            let task = tasks.get("server-extends").unwrap();

            assert_eq!(task.preset, Some(TaskPreset::Server));
            assert_eq!(task.options.cache, TaskOptionCache::Enabled(true));
            assert!(!task.options.interactive);
            assert!(task.options.persistent);
            assert!(!task.options.run_in_ci.is_enabled());
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn utility() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("presets").await;

            let task = tasks.get("utility").unwrap();

            assert_eq!(task.preset, Some(TaskPreset::Utility));
            assert_eq!(task.options.cache, TaskOptionCache::Enabled(false));
            assert!(task.options.interactive);
            assert!(!task.options.persistent);
            assert_eq!(task.options.run_in_ci, TaskOptionRunInCI::Skip);
            assert_eq!(task.options.output_style, Some(TaskOutputStyle::Stream));

            // Custom overrides
            let task = tasks.get("utility-custom").unwrap();

            assert_eq!(task.preset, Some(TaskPreset::Utility));
            assert_eq!(task.options.cache, TaskOptionCache::Enabled(false));
            assert!(!task.options.interactive);
            assert!(!task.options.persistent);
            assert_eq!(task.options.run_in_ci, TaskOptionRunInCI::Skip);
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
                    Input::Glob(stub_glob_input("**/*")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                ]
            );
            assert!(!task.state.empty_inputs);

            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![Input::Glob(stub_glob_input(
                    "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                ))]
            );
            assert!(task.state.empty_inputs);

            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    Input::Glob(stub_glob_input("local/*")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
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
                vec![Input::Glob(stub_glob_input(
                    "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                ))]
            );
            assert!(task.state.empty_inputs);
            assert!(task.state.root_level);

            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![Input::Glob(stub_glob_input(
                    "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                ))]
            );
            assert!(task.state.empty_inputs);
            assert!(task.state.root_level);

            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    Input::Glob(stub_glob_input("local/*")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    ))
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
                    Input::Glob(stub_glob_input("**/*")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    ))
                ]
            );
            assert!(!task.state.empty_inputs);
            assert!(task.state.root_level);

            let task = tasks.get("empty-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![Input::Glob(stub_glob_input(
                    "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                ))]
            );
            assert!(task.state.empty_inputs);
            assert!(task.state.root_level);

            let task = tasks.get("with-inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    Input::Glob(stub_glob_input("local/*")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    ))
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
                    Input::Glob(stub_glob_input("src/**/*")),
                    Input::File(stub_file_input("/workspace-local")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/all.yml")),
                ]
            );
            assert!(!task.state.empty_inputs);

            let task = tasks.get("global-test").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("local.json")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/all.yml")),
                ]
            );
            assert!(!task.state.empty_inputs);

            let task = tasks.get("global-run").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/all.yml")),
                ]
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
                EnvMap::from_iter([
                    ("KEY1".into(), Some("overwrite".into())),
                    ("KEY2".into(), Some("value2".into())),
                    ("LOCAL".into(), Some("true".into())),
                ])
            );

            let task = tasks.get("inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("global")),
                    Input::File(stub_file_input("local")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-merge.yml")),
                ]
            );

            let task = tasks.get("outputs").unwrap();

            assert_eq!(
                task.outputs,
                vec![
                    Output::File(stub_file_output("global")),
                    Output::File(stub_file_output("local")),
                ]
            );

            let task = tasks.get("toolchains").unwrap();

            assert_eq!(task.toolchains, vec!["local", "global"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn append_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-append").await;

            let task = tasks.get("all").unwrap();

            assert_eq!(task.args, vec!["a", "b", "c", "x", "y", "z"]);

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("global:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("local:build").unwrap()),
                ]
            );

            assert_eq!(
                task.env,
                EnvMap::from_iter([
                    ("KEY1".into(), Some("overwrite".into())),
                    ("KEY2".into(), Some("value2".into())),
                    ("LOCAL".into(), Some("true".into())),
                ])
            );

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("global")),
                    Input::File(stub_file_input("local")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-merge.yml")),
                ]
            );

            assert_eq!(
                task.outputs,
                vec![
                    Output::File(stub_file_output("global")),
                    Output::File(stub_file_output("local")),
                ]
            );

            assert_eq!(task.toolchains, vec!["local", "global"]);
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
                EnvMap::from_iter([
                    ("KEY1".into(), Some("value1".into())),
                    ("KEY2".into(), Some("value2".into())),
                    ("LOCAL".into(), Some("true".into())),
                ])
            );

            let task = tasks.get("inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("local")),
                    Input::File(stub_file_input("global")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-merge.yml")),
                ]
            );

            let task = tasks.get("outputs").unwrap();

            assert_eq!(
                task.outputs,
                vec![
                    Output::File(stub_file_output("local")),
                    Output::File(stub_file_output("global")),
                ]
            );

            let task = tasks.get("toolchains").unwrap();

            assert_eq!(task.toolchains, vec!["local", "global"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prepend_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-prepend").await;

            let task = tasks.get("all").unwrap();

            assert_eq!(task.args, vec!["x", "y", "z", "a", "b", "c"]);

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("local:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("global:build").unwrap()),
                ]
            );

            assert_eq!(
                task.env,
                EnvMap::from_iter([
                    ("KEY1".into(), Some("value1".into())),
                    ("KEY2".into(), Some("value2".into())),
                    ("LOCAL".into(), Some("true".into())),
                ])
            );

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("local")),
                    Input::File(stub_file_input("global")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-merge.yml")),
                ]
            );

            assert_eq!(
                task.outputs,
                vec![
                    Output::File(stub_file_output("local")),
                    Output::File(stub_file_output("global")),
                ]
            );

            assert_eq!(task.toolchains, vec!["local", "global"]);
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
                EnvMap::from_iter([
                    ("KEY1".into(), Some("overwrite".into())),
                    ("LOCAL".into(), Some("true".into())),
                ])
            );

            let task = tasks.get("inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("local")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-merge.yml")),
                ]
            );

            let task = tasks.get("outputs").unwrap();

            assert_eq!(task.outputs, vec![Output::File(stub_file_output("local"))]);

            let task = tasks.get("toolchains").unwrap();

            assert_eq!(task.toolchains, vec!["local"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn replace_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-replace").await;

            let task = tasks.get("all").unwrap();

            assert_eq!(task.args, vec!["x", "y", "z"]);

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("local:build").unwrap()
                )]
            );

            assert_eq!(
                task.env,
                EnvMap::from_iter([
                    ("KEY1".into(), Some("overwrite".into())),
                    ("LOCAL".into(), Some("true".into())),
                ])
            );

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("local")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-merge.yml")),
                ]
            );

            assert_eq!(task.outputs, vec![Output::File(stub_file_output("local"))]);

            assert_eq!(task.toolchains, vec!["local"]);
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

            let task = tasks.get("toolchains").unwrap();

            // fallback
            assert_eq!(task.toolchains, ["system"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn replace_empty_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-replace-empty").await;

            let task = tasks.get("all").unwrap();

            assert!(task.args.is_empty());
            assert!(task.deps.is_empty());
            assert!(task.env.is_empty());

            // inherited
            assert_eq!(task.inputs.len(), 2);
            assert!(task.state.empty_inputs);

            assert!(task.outputs.is_empty());

            // fallback
            assert_eq!(task.toolchains, ["system"]);
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
                EnvMap::from_iter([
                    ("KEY1".into(), Some("value1".into())),
                    ("KEY2".into(), Some("value2".into())),
                ])
            );

            let task = tasks.get("inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("global")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-merge.yml")),
                ]
            );
            assert!(!task.state.empty_inputs);

            let task = tasks.get("outputs").unwrap();

            assert_eq!(task.outputs, vec![Output::File(stub_file_output("global"))]);

            let task = tasks.get("toolchains").unwrap();

            assert_eq!(task.toolchains, vec!["global"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_replace_undefined_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-replace-undefined").await;

            let task = tasks.get("all").unwrap();

            assert_eq!(task.command, "noop");
            assert_eq!(task.args, vec!["a", "b", "c"]);

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("global:build").unwrap()
                )]
            );

            assert_eq!(
                task.env,
                EnvMap::from_iter([
                    ("KEY1".into(), Some("value1".into())),
                    ("KEY2".into(), Some("value2".into())),
                ])
            );

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("global")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-merge.yml")),
                ]
            );
            assert!(!task.state.empty_inputs);

            assert_eq!(task.outputs, vec![Output::File(stub_file_output("global"))]);

            assert_eq!(task.args, vec!["a", "b", "c"]);
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
                EnvMap::from_iter([
                    ("KEY1".into(), Some("value1".into())),
                    ("KEY2".into(), Some("value2".into())),
                ])
            );

            let task = tasks.get("inputs").unwrap();

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("global")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-merge.yml")),
                ]
            );

            let task = tasks.get("outputs").unwrap();

            assert_eq!(task.outputs, vec![Output::File(stub_file_output("global"))]);

            let task = tasks.get("toolchains").unwrap();

            assert_eq!(task.toolchains, vec!["global"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn preserve_all_option() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("merge-preserve").await;

            let task = tasks.get("all").unwrap();

            assert_eq!(task.args, vec!["a", "b", "c"]);

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("global:build").unwrap()
                )]
            );

            assert_eq!(
                task.env,
                EnvMap::from_iter([
                    ("KEY1".into(), Some("value1".into())),
                    ("KEY2".into(), Some("value2".into())),
                ])
            );

            assert_eq!(
                task.inputs,
                vec![
                    Input::File(stub_file_input("global")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-merge.yml")),
                ]
            );

            assert_eq!(task.outputs, vec![Output::File(stub_file_output("global"))]);

            assert_eq!(task.toolchains, vec!["global"]);
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
                EnvMap::from_iter([
                    ("SCOPE".into(), Some("project".into())),
                    ("KEY".into(), Some("value".into())),
                ])
            );

            let run = tasks.get("local-run").unwrap();

            assert_eq!(
                run.env,
                EnvMap::from_iter([
                    ("SCOPE".into(), Some("project".into())),
                    ("KEY".into(), Some("value".into())),
                ])
            );

            let test = tasks.get("local-test").unwrap();

            assert_eq!(
                test.env,
                EnvMap::from_iter([
                    ("SCOPE".into(), Some("task".into())),
                    ("KEY".into(), Some("value".into())),
                    ("KEY2".into(), Some("value2".into())),
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
                    Input::Glob(stub_glob_input("**/*")),
                    Input::Glob(stub_glob_input("project/**/*")),
                    Input::File(stub_file_input("/workspace.json")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
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
                    Input::Glob(stub_glob_input("project/**/*")),
                    Input::File(stub_file_input("/workspace.json")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
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
                    Input::Glob(stub_glob_input("local/*")),
                    Input::Glob(stub_glob_input("project/**/*")),
                    Input::File(stub_file_input("/workspace.json")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
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
                EnvMap::from_iter([
                    ("SCOPE".into(), Some("project".into())),
                    ("KEY1".into(), Some("value1".into())),
                    ("KEY2".into(), Some("value2".into())),
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
                EnvMap::from_iter([
                    ("SCOPE".into(), Some("task".into())),
                    ("KEY1".into(), Some("value1".into())),
                    ("KEY2".into(), Some("env-value2".into())),
                    ("EXTRA".into(), Some("123".into())),
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
                    Input::Glob(stub_glob_input("src/**/*")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_options() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("extends").await;
            let task = tasks.get("extend-options").unwrap();

            assert_eq!(task.options.cache, TaskOptionCache::Enabled(false));
            assert!(task.options.run_in_ci.is_enabled());
            assert!(task.options.persistent);
            assert_eq!(task.options.retry_count, 3);
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
                    Input::File(stub_file_input("global-base")),
                    Input::File(stub_file_input("global-extender")),
                    Input::File(stub_file_input("local-base")),
                    Input::File(stub_file_input("local-extender")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-extends.yml")),
                ]
            );

            assert_eq!(task.options.cache, TaskOptionCache::Enabled(true));
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
                    Input::File(stub_file_input("global-base")),
                    Input::File(stub_file_input("local-extender")),
                    Input::Glob(stub_glob_input(
                        "/.moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                    )),
                    Input::File(stub_file_input("/global/tasks/tag-extends.yml")),
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
                EnvMap::from_iter([
                    ("VAR2".into(), Some("global-child".into())),
                    ("VAR3".into(), Some("local-child".into())),
                    ("VAR1".into(), Some("local-parent".into())),
                ])
            );
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
            assert_eq!(task.args, Vec::<TaskArg>::new());
            assert_eq!(task.script.as_ref().unwrap(), "foo --bar baz");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn multi_command() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("scripts").await;
            let task = tasks.get("multi-command").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<TaskArg>::new());
            assert_eq!(task.script.as_ref().unwrap(), "foo --bar baz && qux -abc");

            let task = tasks.get("multi-command-semi").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<TaskArg>::new());
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
            assert_eq!(task.args, Vec::<TaskArg>::new());
            assert_eq!(task.script.as_ref().unwrap(), "foo | bar | baz");

            let task = tasks.get("redirect").unwrap();

            assert_eq!(task.command, "foo");
            assert_eq!(task.args, Vec::<TaskArg>::new());
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

            assert_eq!(task.args, Vec::<TaskArg>::new());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn cannot_disable_shell() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());

            let tasks = container.build_tasks("scripts").await;
            let task = tasks.get("no-shell").unwrap();

            assert_eq!(task.options.shell, Some(true));
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
                assert_eq!(task.args, Vec::<TaskArg>::new());
                assert_eq!(task.script, Some("execute --nix".to_owned()));
            } else {
                assert_eq!(task.command, "noop");
                assert_eq!(task.args, Vec::<TaskArg>::new());
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
                assert_eq!(task.args, Vec::<TaskArg>::new());
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
                assert_eq!(task.args, Vec::<TaskArg>::new());
                assert_eq!(task.script, None);
            }
        }
    }

    mod option_run_in_ci {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn enables_or_disables_based_on_task_type() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());
            let tasks = container.build_tasks("options-runinci").await;

            let task = tasks.get("build-type").unwrap();

            assert_eq!(task.options.run_in_ci, TaskOptionRunInCI::Enabled(true));

            let task = tasks.get("test-type").unwrap();

            assert_eq!(task.options.run_in_ci, TaskOptionRunInCI::Enabled(true));

            let task = tasks.get("run-type").unwrap();

            assert_eq!(task.options.run_in_ci, TaskOptionRunInCI::Enabled(false));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_override_explicit_setting() {
            let sandbox = create_sandbox("builder");
            let container = TasksBuilderContainer::new(sandbox.path());
            let tasks = container.build_tasks("options-runinci").await;

            let task = tasks.get("build-type-custom").unwrap();

            assert_eq!(task.options.run_in_ci, TaskOptionRunInCI::Only);

            let task = tasks.get("test-type-custom").unwrap();

            assert_eq!(task.options.run_in_ci, TaskOptionRunInCI::Skip);

            let task = tasks.get("run-type-custom").unwrap();

            assert_eq!(task.options.run_in_ci, TaskOptionRunInCI::Always);
        }
    }
}
