use moon_config::PartialBunConfig;
use moon_test_utils::{
    Sandbox, assert_snapshot, create_sandbox, create_sandbox_with_config, get_bun_fixture_configs,
    predicates::prelude::*,
};
use std::fs;

fn bun_sandbox() -> Sandbox {
    bun_sandbox_with_config(|_| {})
}

fn bun_sandbox_with_config<C>(callback: C) -> Sandbox
where
    C: FnOnce(&mut PartialBunConfig),
{
    let (workspace_config, mut toolchain_config, tasks_config) = get_bun_fixture_configs();

    if let Some(bun_config) = &mut toolchain_config.bun {
        callback(bun_config);
    }

    let sandbox = create_sandbox_with_config(
        "bun",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    sandbox.enable_git();
    sandbox
}

mod bun {
    use super::*;

    #[test]
    fn runs_self() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:version");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_standard_script() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:standard");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_cjs_files() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:cjs");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_mjs_files() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:mjs");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn supports_top_level_await() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:topLevelAwait");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_zero() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:processExitZero");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:processExitNonZero");
        });

        if cfg!(windows) {
            assert.code(1);
        } else {
            assert_snapshot!(assert.output());
        }
    }

    #[test]
    fn handles_process_exit_code_zero() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:exitCodeZero");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_code_nonzero() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:exitCodeNonZero");
        });

        if cfg!(windows) {
            assert.code(1);
        } else {
            assert_snapshot!(assert.output());
        }
    }

    #[test]
    fn handles_throw_error() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:throwError");
        });

        let output = assert.output();

        // Output contains file paths that we cant snapshot
        assert!(predicate::str::contains("error: Oops").eval(&output));
    }

    #[test]
    fn handles_unhandled_promise() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:unhandledPromise");
        });

        let output = assert.output();

        // Output contains os/arch stuff that we cant snapshot
        assert!(predicate::str::contains("error: Oops").eval(&output));
    }

    #[test]
    fn passes_args_through() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("bun:passthroughArgs")
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

        // Quoting is handled differently between unix and windows,
        // so only check part of the arg string
        assert!(
            predicate::str::contains("Args: -aBc --opt value --optCamel=value foo")
                .eval(&assert.output())
        );
    }

    #[test]
    fn sets_env_vars() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:envVars");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn inherits_moon_env_vars() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:envVarsMoon");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn forces_cache_to_write_only() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:envVarsMoon").arg("--updateCache");
        });

        assert!(predicate::str::contains("MOON_CACHE=write").eval(&assert.output()));
    }

    #[test]
    fn runs_from_project_root() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:runFromProject");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_from_workspace_root() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:runFromWorkspace");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_node_module_bin_from_workspace_root() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:runFromWorkspaceBin");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn retries_on_failure_till_count() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:retryCount");
        });

        let output = assert.output();

        assert!(predicate::str::contains("exit code 1").eval(&output));
    }

    #[test]
    fn can_run_many_targets() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:cjs").arg("bun:mjs");
        });

        let output = assert.output();

        assert!(predicate::str::contains("bun:cjs | stdout").eval(&output));
        assert!(predicate::str::contains("bun:mjs | stdout").eval(&output));
        assert!(predicate::str::contains("bun:cjs | stderr").eval(&output));
        assert!(predicate::str::contains("bun:mjs | stderr").eval(&output));
    }

    #[test]
    fn runs_script_task() {
        let sandbox = bun_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:viaScript");
        });

        assert_snapshot!(assert.output());
    }

    mod package_manager {
        use super::*;

        #[test]
        fn can_install_a_dep() {
            let sandbox = bun_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("packageManager:installDep");
            });

            assert.success();
        }

        #[test]
        fn can_run_a_deps_bin() {
            let sandbox = bun_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("packageManager:runDep");
            });

            assert!(
                predicate::str::contains("All matched files use Prettier code style!")
                    .eval(&assert.output())
            );

            assert.success();
        }

        #[test]
        fn can_run_a_script() {
            let sandbox = bun_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("packageManager:runScript");
            });

            assert!(predicate::str::contains("test").eval(&assert.output()));

            assert.success();
        }
    }

    mod workspace_overrides {
        use super::*;

        #[test]
        fn can_override_version() {
            let sandbox = bun_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("bun:version")
                    .arg("versionOverride:version");
            });

            let output = assert.output();

            assert!(predicate::str::contains("1.2.2").eval(&output));
            assert!(predicate::str::contains("1.1.0").eval(&output));

            assert.success();
        }
    }

    mod affected_files {
        use super::*;

        #[test]
        fn all_files_when_not_affected() {
            let sandbox = bun_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("bun:affectedFiles");
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
            let sandbox = bun_sandbox();

            sandbox.create_file("base/input1.js", "");
            sandbox.create_file("base/input2.js", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("bun:affectedFiles").arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: ./input1.js ./input2.js\n").eval(&output));
            assert!(predicate::str::contains("Env: input1.js,input2.js\n").eval(&output));
        }

        #[test]
        fn sets_args_only() {
            let sandbox = bun_sandbox();

            sandbox.create_file("base/input1.js", "");
            sandbox.create_file("base/input2.js", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("bun:affectedFilesArgs")
                    .arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: ./input1.js ./input2.js\n").eval(&output));
            assert!(predicate::str::contains("Env: \n").eval(&output));
        }

        #[test]
        fn sets_env_var_only() {
            let sandbox = bun_sandbox();

            sandbox.create_file("base/input1.js", "");
            sandbox.create_file("base/input2.js", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("bun:affectedFilesEnvVar")
                    .arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: \n").eval(&output));
            assert!(predicate::str::contains("Env: input1.js,input2.js\n").eval(&output));
        }
    }

    mod infer_tasks {
        use super::*;

        #[test]
        fn inherits_tasks() {
            let sandbox = bun_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("project").arg("scripts");
            });

            assert_snapshot!(assert.output());
        }
    }

    mod interop {
        use super::*;

        #[test]
        fn doesnt_collide_with_node_bun_pm() {
            let sandbox = create_sandbox("bun-node-pm");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg(":help");
            });

            assert.success();
        }
    }
}
