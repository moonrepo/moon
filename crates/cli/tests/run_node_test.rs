use moon_config::PartialToolchainPluginConfig;
use moon_test_utils::{
    Sandbox, assert_snapshot, create_sandbox_with_config, get_node_fixture_configs,
    predicates::prelude::*,
};
use starbase_utils::json::JsonValue;
use std::fs;

fn node_sandbox() -> Sandbox {
    node_sandbox_with_config(|_| {})
}

#[allow(deprecated)]
fn node_sandbox_with_config<C>(callback: C) -> Sandbox
where
    C: FnOnce(&mut PartialToolchainPluginConfig),
{
    let (workspace_config, mut toolchain_config, tasks_config) = get_node_fixture_configs();

    if let Some(node_config) = toolchain_config
        .plugins
        .as_mut()
        .and_then(|cfg| cfg.get_mut("node"))
    {
        callback(node_config);
    }

    let sandbox = create_sandbox_with_config(
        "node",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    sandbox.enable_git();
    sandbox
}

mod node {
    use super::*;

    #[test]
    fn runs_package_managers() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:npm");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_standard_script() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:standard");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_cjs_files() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:cjs");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_mjs_files() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:mjs");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn supports_top_level_await() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:topLevelAwait");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_zero() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:processExitZero");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:processExitNonZero");
        });

        if cfg!(windows) {
            assert.code(1);
        } else {
            assert_snapshot!(assert.output());
        }
    }

    #[test]
    fn handles_process_exit_code_zero() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:exitCodeZero");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_code_nonzero() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:exitCodeNonZero");
        });

        if cfg!(windows) {
            assert.code(1);
        } else {
            assert_snapshot!(assert.output());
        }
    }

    #[test]
    fn handles_throw_error() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:throwError");
        });

        let output = assert.output();

        // Output contains file paths that we cant snapshot
        assert!(predicate::str::contains("Error: Oops").eval(&output));
    }

    #[test]
    fn handles_unhandled_promise() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:unhandledPromise");
        });

        if cfg!(windows) {
            assert.code(1);
        } else {
            assert_snapshot!(assert.output());
        }
    }

    #[test]
    fn passes_args_through() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("node:passthroughArgs")
                .arg("--")
                .arg("-aBc")
                .arg("--opt")
                .arg("value")
                .arg("--optCamel=value")
                .arg("foo")
                .arg("'bar baz'")
                .arg("--opt-kebab")
                .arg("123");
        });

        assert!(
            predicate::str::contains("Args: -aBc --opt value --optCamel=value")
                .eval(&assert.output())
        );
    }

    #[test]
    fn passes_args_to_the_node_bin() {
        let sandbox = node_sandbox_with_config(|cfg| {
            cfg.config.get_or_insert_default().insert(
                "binExecArgs".into(),
                JsonValue::Array(vec![JsonValue::String("--preserve-symlinks".into())]),
            );
        });

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("node:binExecArgs")
                .arg("--")
                .arg("--extraArg");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn sets_env_vars() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:envVars");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn inherits_moon_env_vars() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:envVarsMoon");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn forces_cache_to_write_only() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:envVarsMoon").arg("--updateCache");
        });

        assert!(predicate::str::contains("MOON_CACHE=write").eval(&assert.output()));
    }

    #[test]
    fn runs_from_project_root() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:runFromProject");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_from_workspace_root() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:runFromWorkspace");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_npm_bin_from_workspace_root() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:runFromWorkspaceBin");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn retries_on_failure_till_count() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:retryCount");
        });

        let output = assert.output();

        assert!(predicate::str::contains("exit code 1").eval(&output));
    }

    #[test]
    fn can_run_many_targets() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:cjs").arg("node:mjs");
        });

        let output = assert.output();

        assert!(predicate::str::contains("node:cjs | stdout").eval(&output));
        assert!(predicate::str::contains("node:mjs | stdout").eval(&output));
        assert!(predicate::str::contains("node:cjs | stderr").eval(&output));
        assert!(predicate::str::contains("node:mjs | stderr").eval(&output));
    }

    #[test]
    fn avoids_postinstall_recursion() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("postinstallRecursion:noop");
        });

        let output = assert.output();

        assert!(
            predicate::str::contains("postinstallRecursion:noop")
                .count(1)
                .eval(&output)
        );

        assert.success();
    }

    #[test]
    fn can_exec_global_bin_as_child_process() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:execBinSelf");
        });

        let output = assert.output();

        assert!(predicate::str::contains("execBinSelf").eval(&output));
        assert!(predicate::str::contains("v").eval(&output)); // Version not deterministic
    }

    #[test]
    fn can_exec_global_bin_as_child_process_from_postinstall() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("postinstall:noop");
        });

        assert.success();
    }

    #[test]
    fn runs_script_task() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:viaScript");
        });

        assert_snapshot!(assert.output());
    }

    mod affected_files {
        use super::*;

        #[test]
        fn all_files_when_not_affected() {
            let sandbox = node_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("node:affectedFiles");
            });

            let mut files = fs::read_dir(sandbox.path().join("base"))
                .unwrap()
                .map(|f| {
                    f.unwrap()
                        .path()
                        .strip_prefix(sandbox.path().join("base"))
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned()
                })
                .collect::<Vec<_>>();
            files.sort();

            let args = files
                .clone()
                .into_iter()
                .map(|f| "./".to_owned() + &f)
                .collect::<Vec<_>>()
                .join(" ");
            let envs = files.join(",");

            let output = assert.output();

            assert!(predicate::str::contains(format!("Args: {args}\n")).eval(&output));
            assert!(predicate::str::contains(format!("Env: {envs}\n")).eval(&output));
        }

        #[test]
        fn uses_rel_paths_when_affected() {
            let sandbox = node_sandbox();

            sandbox.create_file("base/input1.js", "");
            sandbox.create_file("base/input2.js", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("node:affectedFiles").arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: ./input1.js ./input2.js\n").eval(&output));
            assert!(predicate::str::contains("Env: input1.js,input2.js\n").eval(&output));
        }

        #[test]
        fn sets_args_only() {
            let sandbox = node_sandbox();

            sandbox.create_file("base/input1.js", "");
            sandbox.create_file("base/input2.js", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("node:affectedFilesArgs")
                    .arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: ./input1.js ./input2.js\n").eval(&output));
            assert!(predicate::str::contains("Env: \n").eval(&output));
        }

        #[test]
        fn sets_env_var_only() {
            let sandbox = node_sandbox();

            sandbox.create_file("base/input1.js", "");
            sandbox.create_file("base/input2.js", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("node:affectedFilesEnvVar")
                    .arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: \n").eval(&output));
            assert!(predicate::str::contains("Env: input1.js,input2.js\n").eval(&output));
        }
    }
}
