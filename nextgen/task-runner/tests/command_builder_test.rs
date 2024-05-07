mod utils;

use moon_action::ActionNode;
use moon_action_context::ActionContext;
use moon_config::TaskOptionAffectedFiles;
use moon_process::Command;
use moon_task::{Target, TargetLocator, Task};
use moon_task_runner::command_builder::CommandBuilder;
use moon_test_utils2::generate_platform_manager_from_sandbox;
use starbase_sandbox::{create_sandbox, Sandbox};
use std::ffi::OsString;
use utils::*;

fn get_env<'a, 'b>(command: &'a Command, key: &'b str) -> Option<&'a str> {
    command
        .env
        .get(&OsString::from(key))
        .map(|v| v.to_str().unwrap())
}

fn get_args(command: &Command) -> Vec<&str> {
    command
        .args
        .iter()
        .map(|arg| arg.to_str().unwrap())
        .collect()
}

async fn build(
    context: ActionContext,
    mut op: impl FnMut(&mut Task, &mut ActionNode),
) -> (Sandbox, Command) {
    let sandbox = create_sandbox("builder");
    sandbox.create_file("apps/project/literal.txt", "");

    let workspace = create_workspace(sandbox.path());
    let project = create_project(sandbox.path());
    let mut task = create_task(&project);
    let mut node = create_node(&task);

    op(&mut task, &mut node);

    let platform = generate_platform_manager_from_sandbox(sandbox.path()).await;

    let mut builder = CommandBuilder::new(&workspace, &project, &task, &node);
    builder.set_platform_manager(&platform);

    (sandbox, builder.build(&context).await.unwrap())
}

mod command_builder {
    use super::*;

    #[tokio::test]
    async fn sets_cwd_to_project_root() {
        let (sandbox, command) = build(ActionContext::default(), |_, _| {}).await;

        assert_eq!(command.cwd, Some(sandbox.path().join("apps/project")));
    }

    #[tokio::test]
    async fn sets_cwd_to_workspace_root() {
        let (sandbox, command) = build(ActionContext::default(), |task, _| {
            task.options.run_from_workspace_root = true;
        })
        .await;

        assert_eq!(command.cwd, Some(sandbox.path().to_path_buf()));
    }

    mod args {
        use super::*;

        #[tokio::test]
        async fn inherits_task_args() {
            let (_sandbox, command) = build(ActionContext::default(), |_, _| {}).await;

            assert_eq!(get_args(&command), vec!["arg", "--opt"]);
        }

        #[tokio::test]
        async fn inherits_when_a_task_dep() {
            let (_sandbox, command) = build(ActionContext::default(), |_, node| {
                if let ActionNode::RunTask(inner) = node {
                    inner.args.push("extra-arg".into());
                }
            })
            .await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "extra-arg"]);
        }

        #[tokio::test]
        async fn inherits_passthrough_args_when_a_primary_target() {
            let mut context = ActionContext::default();
            context.passthrough_args.push("--passthrough".into());
            context
                .primary_targets
                .insert(Target::new("project", "task").unwrap());

            let (_sandbox, command) = build(context, |_, _| {}).await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "--passthrough"]);
        }

        #[tokio::test]
        async fn inherits_passthrough_args_when_an_all_initial_target() {
            let mut context = ActionContext::default();
            context.passthrough_args.push("--passthrough".into());
            context
                .initial_targets
                .insert(TargetLocator::Qualified(Target::parse(":task").unwrap()));

            dbg!(&context);

            let (_sandbox, command) = build(context, |_, _| {}).await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "--passthrough"]);
        }

        #[tokio::test]
        async fn doesnt_inherit_passthrough_args_when_not_a_target() {
            let mut context = ActionContext::default();
            context.passthrough_args.push("--passthrough".into());
            context
                .primary_targets
                .insert(Target::new("other-project", "task").unwrap());

            let (_sandbox, command) = build(context, |_, _| {}).await;

            assert_eq!(get_args(&command), vec!["arg", "--opt"]);
        }

        #[tokio::test]
        async fn passthrough_comes_after_node_deps() {
            let mut context = ActionContext::default();
            context.passthrough_args.push("--passthrough".into());
            context
                .primary_targets
                .insert(Target::new("project", "task").unwrap());

            let (_sandbox, command) = build(context, |_, node| {
                if let ActionNode::RunTask(inner) = node {
                    inner.args.push("extra-arg".into());
                }
            })
            .await;

            assert_eq!(
                get_args(&command),
                vec!["arg", "--opt", "extra-arg", "--passthrough"]
            );
        }
    }

    mod env {
        use super::*;

        #[tokio::test]
        async fn sets_pwd() {
            let (sandbox, command) = build(ActionContext::default(), |_, _| {}).await;

            assert_eq!(
                get_env(&command, "PWD").unwrap(),
                sandbox.path().join("apps/project").to_str().unwrap()
            );
        }

        #[tokio::test]
        async fn inherits_task_env() {
            let (_sandbox, command) = build(ActionContext::default(), |_, _| {}).await;

            assert_eq!(get_env(&command, "KEY").unwrap(), "value");
        }

        #[tokio::test]
        async fn inherits_when_a_task_dep() {
            let (_sandbox, command) = build(ActionContext::default(), |_, node| {
                if let ActionNode::RunTask(inner) = node {
                    inner.env.insert("ANOTHER".into(), "value".into());
                }
            })
            .await;

            assert_eq!(get_env(&command, "ANOTHER").unwrap(), "value");
        }

        #[tokio::test]
        async fn can_overwrite_env_via_task_dep() {
            let (_sandbox, command) = build(ActionContext::default(), |_, node| {
                if let ActionNode::RunTask(inner) = node {
                    inner.env.insert("KEY".into(), "overwritten".into());
                }
            })
            .await;

            assert_eq!(get_env(&command, "KEY").unwrap(), "overwritten");
        }

        #[tokio::test]
        async fn cannot_overwrite_built_in_env() {
            let (_sandbox, command) = build(ActionContext::default(), |_, node| {
                if let ActionNode::RunTask(inner) = node {
                    inner.env.insert("PWD".into(), "overwritten".into());
                    inner
                        .env
                        .insert("MOON_PROJECT_ID".into(), "overwritten".into());
                    inner
                        .env
                        .insert("PROTO_VERSION".into(), "overwritten".into());
                }
            })
            .await;

            assert_ne!(get_env(&command, "PWD").unwrap(), "overwritten");
            assert_ne!(get_env(&command, "MOON_PROJECT_ID").unwrap(), "overwritten");
            assert_ne!(get_env(&command, "PROTO_VERSION").unwrap(), "overwritten");
        }
    }

    mod shell {
        use super::*;

        #[tokio::test]
        async fn doesnt_use_a_shell_by_default() {
            let (_sandbox, command) = build(ActionContext::default(), |_, _| {}).await;

            assert!(command.shell.is_none());
        }

        #[tokio::test]
        async fn sets_default_shell() {
            let (_sandbox, command) = build(ActionContext::default(), |task, _| {
                task.options.shell = Some(true);
            })
            .await;

            assert!(command.shell.is_some());
        }

        #[cfg(unix)]
        #[tokio::test]
        async fn can_set_unix_shell() {
            let (_sandbox, command) = build(ActionContext::default(), |task, _| {
                task.options.shell = Some(true);
                task.options.unix_shell = Some(moon_config::TaskUnixShell::Elvish);
            })
            .await;

            assert!(command.shell.unwrap().bin.to_string_lossy().contains("elv"));
        }

        #[cfg(windows)]
        #[tokio::test]
        async fn can_set_windows_shell() {
            let (_sandbox, command) = build(ActionContext::default(), |task, _| {
                task.options.shell = Some(true);
                task.options.windows_shell = Some(moon_config::TaskWindowsShell::Bash);
            })
            .await;

            assert!(command
                .shell
                .unwrap()
                .bin
                .to_string_lossy()
                .contains("bash"));
        }
    }

    mod affected {
        use super::*;

        #[tokio::test]
        async fn does_nothing_if_option_not_set() {
            let (_sandbox, command) = build(ActionContext::default(), |_, _| {}).await;

            assert!(get_env(&command, "MOON_AFFECTED_FILES").is_none());
        }

        #[tokio::test]
        async fn includes_touched_in_args() {
            let mut context = ActionContext::default();
            context.affected_only = true;
            context.touched_files.insert("apps/project/file.txt".into());

            let (_sandbox, command) = build(context, |task, _| {
                task.options.affected_files = Some(TaskOptionAffectedFiles::Args);
            })
            .await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "./file.txt"]);
        }

        #[tokio::test]
        async fn fallsback_to_dot_in_args_when_no_match() {
            let mut context = ActionContext::default();
            context.affected_only = true;
            context
                .touched_files
                .insert("apps/project/other.txt".into());

            let (_sandbox, command) = build(context, |task, _| {
                task.options.affected_files = Some(TaskOptionAffectedFiles::Args);
            })
            .await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "."]);
        }

        #[tokio::test]
        async fn includes_touched_in_env() {
            let mut context = ActionContext::default();
            context.affected_only = true;
            context.touched_files.insert("apps/project/file.txt".into());

            let (_sandbox, command) = build(context, |task, _| {
                task.options.affected_files = Some(TaskOptionAffectedFiles::Env);
            })
            .await;

            assert_eq!(
                get_env(&command, "MOON_AFFECTED_FILES").unwrap(),
                "file.txt"
            );
        }

        #[tokio::test]
        async fn fallsback_to_dot_in_env_when_no_match() {
            let mut context = ActionContext::default();
            context.affected_only = true;
            context
                .touched_files
                .insert("apps/project/other.txt".into());

            let (_sandbox, command) = build(context, |task, _| {
                task.options.affected_files = Some(TaskOptionAffectedFiles::Env);
            })
            .await;

            assert_eq!(get_env(&command, "MOON_AFFECTED_FILES").unwrap(), ".");
        }

        #[tokio::test]
        async fn can_use_inputs_directly_when_not_affected() {
            let (_sandbox, command) = build(ActionContext::default(), |task, _| {
                task.options.affected_files = Some(TaskOptionAffectedFiles::Args);
                task.options.affected_pass_inputs = true;
            })
            .await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "./literal.txt"]);
        }
    }
}
