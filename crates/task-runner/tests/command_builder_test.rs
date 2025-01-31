#![allow(clippy::field_reassign_with_default)]

mod utils;

use moon_action::ActionNode;
use moon_action_context::ActionContext;
use moon_affected::Affected;
use moon_config::TaskOptionAffectedFiles;
use moon_process::Command;
use moon_task::Target;
use std::ffi::OsString;
use utils::*;

fn get_env<'a>(command: &'a Command, key: &str) -> Option<&'a str> {
    command
        .env
        .get(&OsString::from(key))
        .map(|v| v.as_ref().unwrap().to_str().unwrap())
}

fn get_args(command: &Command) -> Vec<&str> {
    command
        .args
        .iter()
        .map(|arg| arg.to_str().unwrap())
        .collect()
}

mod command_builder {
    use super::*;

    #[tokio::test]
    async fn sets_cwd_to_project_root() {
        let container = TaskRunnerContainer::new("builder", "base").await;
        let command = container.create_command(ActionContext::default()).await;

        assert_eq!(
            command.cwd.as_deref(),
            Some(container.sandbox.path().join("project").as_os_str())
        );
    }

    #[tokio::test]
    async fn sets_cwd_to_workspace_root() {
        let container = TaskRunnerContainer::new("builder", "base").await;
        let command = container
            .create_command_with_config(ActionContext::default(), |task, _| {
                task.options.run_from_workspace_root = true;
            })
            .await;

        assert_eq!(
            command.cwd.as_deref(),
            Some(container.sandbox.path().as_os_str())
        );
    }

    mod args {
        use super::*;

        #[tokio::test]
        async fn inherits_task_args() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container.create_command(ActionContext::default()).await;

            assert_eq!(get_args(&command), vec!["arg", "--opt"]);
        }

        #[tokio::test]
        async fn inherits_when_a_task_dep() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container
                .create_command_with_config(ActionContext::default(), |_, node| {
                    if let ActionNode::RunTask(inner) = node {
                        inner.args.push("extra-arg".into());
                    }
                })
                .await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "extra-arg"]);
        }

        #[tokio::test]
        async fn inherits_passthrough_args_when_a_primary_target() {
            let container = TaskRunnerContainer::new("builder", "base").await;

            let mut context = ActionContext::default();
            context.passthrough_args.push("--passthrough".into());
            context
                .primary_targets
                .insert(Target::new("project", "base").unwrap());

            let command = container.create_command(context).await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "--passthrough"]);
        }

        #[tokio::test]
        async fn inherits_passthrough_args_when_an_all_initial_target() {
            let container = TaskRunnerContainer::new("builder", "base").await;

            let mut context = ActionContext::default();
            context.passthrough_args.push("--passthrough".into());
            context
                .initial_targets
                .insert(Target::parse(":base").unwrap());

            let command = container.create_command(context).await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "--passthrough"]);
        }

        #[tokio::test]
        async fn doesnt_inherit_passthrough_args_when_not_a_target() {
            let container = TaskRunnerContainer::new("builder", "base").await;

            let mut context = ActionContext::default();
            context.passthrough_args.push("--passthrough".into());
            context
                .primary_targets
                .insert(Target::new("other-project", "base").unwrap());

            let command = container.create_command(context).await;

            assert_eq!(get_args(&command), vec!["arg", "--opt"]);
        }

        #[tokio::test]
        async fn passthrough_comes_after_node_deps() {
            let container = TaskRunnerContainer::new("builder", "base").await;

            let mut context = ActionContext::default();
            context.passthrough_args.push("--passthrough".into());
            context
                .primary_targets
                .insert(Target::new("project", "base").unwrap());

            let command = container
                .create_command_with_config(context, |_, node| {
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
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container.create_command(ActionContext::default()).await;

            assert_eq!(
                get_env(&command, "PWD").unwrap(),
                container.sandbox.path().join("project").to_str().unwrap()
            );
        }

        #[tokio::test]
        async fn inherits_task_env() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container.create_command(ActionContext::default()).await;

            assert_eq!(get_env(&command, "KEY").unwrap(), "value");
        }

        #[tokio::test]
        async fn inherits_when_a_task_dep() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container
                .create_command_with_config(ActionContext::default(), |_, node| {
                    if let ActionNode::RunTask(inner) = node {
                        inner.env.insert("ANOTHER".into(), "value".into());
                    }
                })
                .await;

            assert_eq!(get_env(&command, "ANOTHER").unwrap(), "value");
        }

        #[tokio::test]
        async fn can_overwrite_env_via_task_dep() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container
                .create_command_with_config(ActionContext::default(), |_, node| {
                    if let ActionNode::RunTask(inner) = node {
                        inner.env.insert("KEY".into(), "overwritten".into());
                    }
                })
                .await;

            assert_eq!(get_env(&command, "KEY").unwrap(), "overwritten");
        }

        #[tokio::test]
        async fn cannot_overwrite_built_in_env() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container
                .create_command_with_config(ActionContext::default(), |_, node| {
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
        async fn uses_a_shell_by_default_for_system_task() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container.create_command(ActionContext::default()).await;

            assert!(command.shell.is_some());
        }

        #[tokio::test]
        async fn sets_default_shell() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container
                .create_command_with_config(ActionContext::default(), |task, _| {
                    task.options.shell = Some(true);
                })
                .await;

            assert!(command.shell.is_some());
        }

        #[cfg(unix)]
        #[tokio::test]
        async fn can_set_unix_shell() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container
                .create_command_with_config(ActionContext::default(), |task, _| {
                    task.options.shell = Some(true);
                    task.options.unix_shell = Some(moon_config::TaskUnixShell::Elvish);
                })
                .await;

            assert!(command.shell.unwrap().bin.to_string_lossy().contains("elv"));
        }

        #[cfg(windows)]
        #[tokio::test]
        async fn can_set_windows_shell() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container
                .create_command_with_config(ActionContext::default(), |task, _| {
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
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container.create_command(ActionContext::default()).await;

            assert!(get_env(&command, "MOON_AFFECTED_FILES").is_none());
        }

        #[tokio::test]
        async fn includes_touched_in_args() {
            let container = TaskRunnerContainer::new("builder", "base").await;

            let mut context = ActionContext::default();
            context.affected = Some(Affected::default());
            context.touched_files.insert("project/file.txt".into());

            let command = container
                .create_command_with_config(context, |task, _| {
                    task.options.affected_files = Some(TaskOptionAffectedFiles::Args);
                })
                .await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "./file.txt"]);
        }

        #[tokio::test]
        async fn fallsback_to_dot_in_args_when_no_match() {
            let container = TaskRunnerContainer::new("builder", "base").await;

            let mut context = ActionContext::default();
            context.affected = Some(Affected::default());
            context.touched_files.insert("project/other.txt".into());

            let command = container
                .create_command_with_config(context, |task, _| {
                    task.options.affected_files = Some(TaskOptionAffectedFiles::Args);
                })
                .await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "."]);
        }

        #[tokio::test]
        async fn includes_touched_in_env() {
            let container = TaskRunnerContainer::new("builder", "base").await;

            let mut context = ActionContext::default();
            context.affected = Some(Affected::default());
            context.touched_files.insert("project/file.txt".into());

            let command = container
                .create_command_with_config(context, |task, _| {
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
            let container = TaskRunnerContainer::new("builder", "base").await;

            let mut context = ActionContext::default();
            context.affected = Some(Affected::default());
            context.touched_files.insert("project/other.txt".into());

            let command = container
                .create_command_with_config(context, |task, _| {
                    task.options.affected_files = Some(TaskOptionAffectedFiles::Env);
                })
                .await;

            assert_eq!(get_env(&command, "MOON_AFFECTED_FILES").unwrap(), ".");
        }

        #[tokio::test]
        async fn can_use_inputs_directly_when_not_affected() {
            let container = TaskRunnerContainer::new("builder", "base").await;
            let command = container
                .create_command_with_config(ActionContext::default(), |task, _| {
                    task.options.affected_files = Some(TaskOptionAffectedFiles::Args);
                    task.options.affected_pass_inputs = true;
                })
                .await;

            assert_eq!(get_args(&command), vec!["arg", "--opt", "./input.txt"]);
        }

        #[tokio::test]
        async fn quotes_files_with_special_chars() {
            let container = TaskRunnerContainer::new("builder", "base").await;

            let mut context = ActionContext::default();
            context.affected = Some(Affected::default());
            context.touched_files.insert("project/file.txt".into());
            context.touched_files.insert("project/routes/*.ts".into());
            context
                .touched_files
                .insert("project/routes/[id].ts".into());
            context
                .touched_files
                .insert("project/routes/$slug.tsx".into());
            context
                .touched_files
                .insert("project/routes/+page.svelte".into());

            let command = container
                .create_command_with_config(context, |task, _| {
                    task.options.affected_files = Some(TaskOptionAffectedFiles::Args);
                })
                .await;

            assert_eq!(
                get_args(&command),
                vec![
                    "arg",
                    "--opt",
                    "./file.txt",
                    "\"./routes/$slug.tsx\"",
                    "\"./routes/*.ts\"",
                    "\"./routes/+page.svelte\"",
                    "\"./routes/[id].ts\""
                ]
            );
        }
    }
}
