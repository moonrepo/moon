use moon_action::{ActionStatus, Operation};
use moon_actions::plugins::*;
use moon_env_var::GlobalEnvBag;
use moon_pdk_api::{CacheInput, ExecCommand, ExecCommandInput, VirtualPath};
use moon_test_utils2::WorkspaceMocker;
use starbase_sandbox::{Sandbox, assert_snapshot, create_empty_sandbox};
use starbase_utils::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn create_workspace() -> (Sandbox, WorkspaceMocker) {
    let sandbox = create_empty_sandbox();
    let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

    (sandbox, mocker)
}

mod plugin_commands {
    use super::*;

    mod exec_one {
        use super::*;

        #[tokio::test]
        async fn handles_success_failure() {
            let (_, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            let ops = exec_plugin_command(
                ctx.clone(),
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["success"])),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let exec = ops.first().unwrap().get_exec_output().unwrap();

            assert_eq!(exec.exit_code.unwrap(), 0);
            assert_eq!(exec.stdout.as_deref().unwrap().trim(), "success");

            let ops = exec_plugin_command(
                ctx.clone(),
                &ExecCommand::new(ExecCommandInput::pipe("exit", ["3"])).allow_failure(),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let exec = ops.first().unwrap().get_exec_output().unwrap();

            assert_eq!(exec.exit_code.unwrap(), 3);
        }

        #[tokio::test]
        #[should_panic(expected = "exit code 3")]
        async fn errors_for_nonzero() {
            let (_, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            exec_plugin_command(
                ctx.clone(),
                &ExecCommand::new(ExecCommandInput::pipe("exit", ["3"])),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();
        }

        #[cfg(unix)]
        #[tokio::test]
        async fn can_set_env() {
            let (_, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            let mut input = ExecCommandInput::pipe("echo", ["$TEST"]);
            input.env.insert("TEST".into(), "value".into());

            let ops = exec_plugin_command(
                ctx,
                &ExecCommand::new(input),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let exec = ops.first().unwrap().get_exec_output().unwrap();

            assert_eq!(exec.exit_code.unwrap(), 0);
            assert_eq!(exec.stdout.as_deref().unwrap().trim(), "value");
        }

        #[tokio::test]
        async fn can_set_cwd() {
            let (sandbox, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            // Current dir
            let mut input = ExecCommandInput::pipe("echo", ["$PWD"]);
            input.cwd = Some(VirtualPath::Real(sandbox.path().into()));

            let ops = exec_plugin_command(
                ctx.clone(),
                &ExecCommand::new(input),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let exec = ops.first().unwrap().get_exec_output().unwrap();

            assert_eq!(exec.exit_code.unwrap(), 0);

            if cfg!(unix) {
                assert_eq!(
                    exec.stdout.as_deref().unwrap().trim(),
                    sandbox.path().to_str().unwrap()
                );
            }

            // Custom dir
            sandbox.create_file("subdir/file", "");

            let mut input = ExecCommandInput::pipe("echo", ["$PWD"]);
            input.cwd = Some(VirtualPath::Real(sandbox.path().join("subdir")));

            let ops = exec_plugin_command(
                ctx.clone(),
                &ExecCommand::new(input),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let exec = ops.first().unwrap().get_exec_output().unwrap();

            assert_eq!(exec.exit_code.unwrap(), 0);

            if cfg!(unix) {
                assert_eq!(
                    exec.stdout.as_deref().unwrap().trim(),
                    sandbox.path().join("subdir").to_str().unwrap()
                );
            }

            // Custom dir via options
            let ops = exec_plugin_command(
                ctx,
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["$PWD"])),
                &ExecCommandOptions {
                    working_dir: Some(sandbox.path().join("subdir")),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

            let exec = ops.first().unwrap().get_exec_output().unwrap();

            assert_eq!(exec.exit_code.unwrap(), 0);

            if cfg!(unix) {
                assert_eq!(
                    exec.stdout.as_deref().unwrap().trim(),
                    sandbox.path().join("subdir").to_str().unwrap()
                );
            }
        }

        #[tokio::test]
        async fn handles_on_exec() {
            let (_, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            let arg = Arc::new(Mutex::new(String::new()));
            let arg2 = arg.clone();

            exec_plugin_command(
                ctx,
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["test"])),
                &ExecCommandOptions {
                    on_exec: Some(Arc::new(move |cmd, _| {
                        arg2.lock().unwrap().push_str(&cmd.command.args[0]);
                        Ok(())
                    })),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

            assert_eq!(arg.lock().unwrap().as_str(), "test");
        }

        #[tokio::test]
        async fn will_retry_on_failure() {
            let (_, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            let ops = exec_plugin_command(
                ctx,
                &ExecCommand::new(ExecCommandInput::pipe("exit", ["1"]))
                    .allow_failure()
                    .retry_count(2),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            assert_eq!(ops.len(), 3);
        }

        #[tokio::test]
        async fn doesnt_retry_on_success() {
            let (_, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            let ops = exec_plugin_command(
                ctx,
                &ExecCommand::new(ExecCommandInput::pipe("exit", ["0"]))
                    .allow_failure()
                    .retry_count(2),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            assert_eq!(ops.len(), 1);
        }
    }

    mod exec_many {
        use super::*;

        fn get_order(ops: Vec<Operation>) -> Vec<String> {
            ops.into_iter()
                .map(|op| {
                    op.get_exec_output()
                        .unwrap()
                        .stdout
                        .as_ref()
                        .unwrap()
                        .trim()
                        .to_owned()
                })
                .collect()
        }

        #[tokio::test]
        async fn runs_serial_in_order() {
            let (_, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            let ops = exec_plugin_commands(
                "toolchain",
                ctx.clone(),
                vec![
                    ExecCommand::new(ExecCommandInput::pipe("echo", ["1"])),
                    ExecCommand::new(ExecCommandInput::pipe("echo", ["2"])),
                    ExecCommand::new(ExecCommandInput::pipe("echo", ["3"])),
                    ExecCommand::new(ExecCommandInput::pipe("echo", ["4"])),
                    ExecCommand::new(ExecCommandInput::pipe("echo", ["5"])),
                ],
                ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            assert_eq!(get_order(ops), ["1", "2", "3", "4", "5"]);
        }

        // #[tokio::test]
        // async fn runs_parallel_in_any_order() {
        //     let (_, ws) = create_workspace();
        //     let ctx = Arc::new(ws.mock_app_context());

        //     let ops = exec_plugin_commands(
        //         "toolchain",
        //         ctx.clone(),
        //         vec![
        //             ExecCommand::new(ExecCommandInput::pipe("echo", ["1"])).parallel(),
        //             ExecCommand::new(ExecCommandInput::pipe("echo", ["2"])).parallel(),
        //             ExecCommand::new(ExecCommandInput::pipe("echo", ["3"])).parallel(),
        //             ExecCommand::new(ExecCommandInput::pipe("echo", ["4"])).parallel(),
        //             ExecCommand::new(ExecCommandInput::pipe("echo", ["5"])).parallel(),
        //             ExecCommand::new(ExecCommandInput::pipe("echo", ["6"])).parallel(),
        //             ExecCommand::new(ExecCommandInput::pipe("echo", ["7"])).parallel(),
        //             ExecCommand::new(ExecCommandInput::pipe("echo", ["8"])).parallel(),
        //             ExecCommand::new(ExecCommandInput::pipe("echo", ["9"])).parallel(),
        //             ExecCommand::new(ExecCommandInput::pipe("echo", ["10"])).parallel(),
        //         ],
        //         ExecCommandOptions::default(),
        //     )
        //     .await
        //     .unwrap();

        //     assert_ne!(
        //         get_order(ops),
        //         ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]
        //     );
        // }

        #[tokio::test]
        async fn runs_parallel_after_serial() {
            let (_, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            let ops = exec_plugin_commands(
                "toolchain",
                ctx.clone(),
                vec![
                    ExecCommand::new(ExecCommandInput::pipe("echo", ["1"])).parallel(),
                    ExecCommand::new(ExecCommandInput::pipe("echo", ["2"])),
                    ExecCommand::new(ExecCommandInput::pipe("echo", ["3"])).parallel(),
                    ExecCommand::new(ExecCommandInput::pipe("echo", ["4"])),
                    ExecCommand::new(ExecCommandInput::pipe("echo", ["5"])).parallel(),
                ],
                ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let order = get_order(ops);

            assert_eq!(order[0], "2");
            assert_eq!(order[1], "4");
        }
    }

    mod cache {
        use super::*;

        fn get_hashes(root: &Path) -> Vec<PathBuf> {
            fs::read_dir(root.join(".moon/cache/hashes"))
                .unwrap()
                .map(|dir| dir.unwrap().path())
                .collect::<Vec<_>>()
        }

        #[tokio::test]
        async fn doesnt_create_cache_if_disabled() {
            let (sandbox, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            exec_plugin_command(
                ctx,
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"])),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            assert_eq!(get_hashes(sandbox.path()).len(), 0);
        }

        #[tokio::test]
        async fn creates_cache_if_enabled() {
            let (sandbox, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            exec_plugin_command(
                ctx,
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"])).cache("key"),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            assert_eq!(get_hashes(sandbox.path()).len(), 1);
        }

        #[tokio::test]
        async fn skips_subsequent_execs() {
            let (_, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            exec_plugin_command(
                ctx.clone(),
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"])).cache("key"),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let ops = exec_plugin_command(
                ctx,
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"])).cache("key"),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let op = ops.first().unwrap();

            assert_eq!(op.status, ActionStatus::Skipped);
        }

        #[tokio::test]
        async fn differentiates_between_keys() {
            let (sandbox, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            exec_plugin_command(
                ctx.clone(),
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"])).cache("key1"),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let ops = exec_plugin_command(
                ctx,
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"])).cache("key2"),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let op = ops.first().unwrap();

            assert_ne!(op.status, ActionStatus::Skipped);

            assert_eq!(get_hashes(sandbox.path()).len(), 2);
        }

        #[tokio::test]
        async fn gathers_env_inputs() {
            let (sandbox, ws) = create_workspace();
            let ctx = Arc::new(ws.mock_app_context());

            let bag = GlobalEnvBag::instance();
            bag.set("EXISTS", "value");

            exec_plugin_command(
                ctx,
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"]))
                    .cache("key")
                    .inputs(vec![
                        CacheInput::EnvVar("EXISTS".into()),
                        CacheInput::EnvVar("MISSING".into()),
                    ]),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            bag.remove("EXISTS");

            let hashes = get_hashes(sandbox.path());
            let data: json::JsonValue = json::read_file(&hashes[0]).unwrap();

            assert_snapshot!(json::format(&data, true).unwrap());
        }

        #[tokio::test]
        async fn gathers_file_inputs_by_size() {
            let (sandbox, ws) = create_workspace();
            sandbox.create_file("input.txt", "test");

            exec_plugin_command(
                Arc::new(ws.mock_app_context()),
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"]))
                    .cache("key")
                    .inputs(vec![
                        CacheInput::FileSize(VirtualPath::Real(sandbox.path().join("input.txt"))),
                        CacheInput::FileSize(VirtualPath::Real(sandbox.path().join("missing.txt"))),
                    ]),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let hashes = get_hashes(sandbox.path());
            let data: json::JsonValue = json::read_file(&hashes[0]).unwrap();

            assert_snapshot!(json::format(&data, true).unwrap());
        }

        #[tokio::test]
        async fn gathers_file_inputs_by_size_instead_of_hash_if_no_vcs() {
            let (sandbox, ws) = create_workspace();
            sandbox.create_file("input.txt", "test");

            exec_plugin_command(
                Arc::new(ws.mock_app_context()),
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"]))
                    .cache("key")
                    .inputs(vec![
                        CacheInput::FileHash(VirtualPath::Real(sandbox.path().join("input.txt"))),
                        CacheInput::FileHash(VirtualPath::Real(sandbox.path().join("missing.txt"))),
                    ]),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let hashes = get_hashes(sandbox.path());
            let data: json::JsonValue = json::read_file(&hashes[0]).unwrap();

            assert_snapshot!(json::format(&data, true).unwrap());
        }

        #[tokio::test]
        async fn gathers_file_inputs_by_hash() {
            let (sandbox, ws) = create_workspace();
            sandbox.enable_git();
            sandbox.create_file("input.txt", "test");

            exec_plugin_command(
                Arc::new(ws.mock_app_context()),
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"]))
                    .cache("key")
                    .inputs(vec![
                        CacheInput::FileHash(VirtualPath::Real(sandbox.path().join("input.txt"))),
                        CacheInput::FileHash(VirtualPath::Real(sandbox.path().join("missing.txt"))),
                    ]),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let hashes = get_hashes(sandbox.path());
            let data: json::JsonValue = json::read_file(&hashes[0]).unwrap();

            assert_snapshot!(json::format(&data, true).unwrap());
        }

        #[tokio::test]
        async fn gathers_file_inputs_by_timestamp() {
            let (sandbox, ws) = create_workspace();
            sandbox.create_file("input.txt", "test");

            exec_plugin_command(
                Arc::new(ws.mock_app_context()),
                &ExecCommand::new(ExecCommandInput::pipe("echo", ["cache"]))
                    .cache("key")
                    .inputs(vec![
                        CacheInput::FileTimestamp(VirtualPath::Real(
                            sandbox.path().join("input.txt"),
                        )),
                        CacheInput::FileTimestamp(VirtualPath::Real(
                            sandbox.path().join("missing.txt"),
                        )),
                    ]),
                &ExecCommandOptions::default(),
            )
            .await
            .unwrap();

            let hashes = get_hashes(sandbox.path());
            let data: json::JsonValue = json::read_file(&hashes[0]).unwrap();

            if let json::JsonValue::String(inner) = &data[0]["inputFiles"]["input.txt"] {
                assert!(inner.starts_with("timestamp:"));
            } else {
                panic!();
            }
        }
    }
}
