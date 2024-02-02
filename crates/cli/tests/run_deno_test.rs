use moon_config::PartialDenoConfig;
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_deno_fixture_configs, predicates::prelude::*,
    Sandbox,
};

fn deno_sandbox() -> Sandbox {
    deno_sandbox_with_config(|_| {})
}

fn deno_sandbox_with_config<C>(callback: C) -> Sandbox
where
    C: FnOnce(&mut PartialDenoConfig),
{
    let (workspace_config, mut toolchain_config, tasks_config) = get_deno_fixture_configs();

    if let Some(deno_config) = &mut toolchain_config.deno {
        callback(deno_config);
    }

    let sandbox = create_sandbox_with_config(
        "deno",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    sandbox.enable_git();
    sandbox
}

mod deno {
    use super::*;

    #[test]
    fn runs_self() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:version");
        });

        let output = assert.output();

        // Output contains arch info
        assert!(predicate::str::contains("deno 1.40.0").eval(&output));
    }

    #[test]
    fn runs_standard_script() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:standard");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn supports_top_level_await() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:topLevelAwait");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_code_zero() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:exitCodeZero");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_code_nonzero() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:exitCodeNonZero");
        });

        if cfg!(windows) {
            assert.code(1);
        } else {
            assert_snapshot!(assert.output());
        }
    }

    #[test]
    fn handles_throw_error() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:throwError");
        });

        let output = assert.output();

        // Output contains file paths that we cant snapshot
        assert!(predicate::str::contains("Error: Oops").eval(&output));
    }

    #[test]
    fn handles_unhandled_promise() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:unhandledPromise");
        });

        if cfg!(windows) {
            assert.code(1);
        } else {
            assert_snapshot!(assert.output());
        }
    }

    #[test]
    fn passes_args_through() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("deno:passthroughArgs")
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
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:envVars");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn inherits_moon_env_vars() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:envVarsMoon");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn forces_cache_to_write_only() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:envVarsMoon").arg("--updateCache");
        });

        assert!(predicate::str::contains("MOON_CACHE=write").eval(&assert.output()));
    }

    #[test]
    fn runs_from_project_root() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:runFromProject");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_from_workspace_root() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:runFromWorkspace");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn retries_on_failure_till_count() {
        let sandbox = deno_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("deno:retryCount");
        });

        let output = assert.output();

        assert!(predicate::str::contains("1 exit code").eval(&output));
    }

    mod workspace_overrides {
        use super::*;

        #[test]
        fn can_override_version() {
            let sandbox = deno_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("deno:version")
                    .arg("versionOverride:version");
            });

            let output = assert.output();

            assert!(predicate::str::contains("1.40.0").eval(&output));
            assert!(predicate::str::contains("1.30.0").eval(&output));

            assert.success();
        }
    }

    mod affected_files {
        use super::*;

        #[test]
        fn uses_dot_when_not_affected() {
            let sandbox = deno_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("deno:affectedFiles");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: .\n").eval(&output));
            assert!(predicate::str::contains("Env: .\n").eval(&output));
        }

        #[test]
        fn uses_rel_paths_when_affected() {
            let sandbox = deno_sandbox();

            sandbox.create_file("base/input1.ts", "");
            sandbox.create_file("base/input2.ts", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("deno:affectedFiles").arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: ./input1.ts ./input2.ts\n").eval(&output));
            assert!(predicate::str::contains("Env: input1.ts,input2.ts\n").eval(&output));
        }

        #[test]
        fn sets_args_only() {
            let sandbox = deno_sandbox();

            sandbox.create_file("base/input1.ts", "");
            sandbox.create_file("base/input2.ts", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("deno:affectedFilesArgs")
                    .arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: ./input1.ts ./input2.ts\n").eval(&output));
            assert!(predicate::str::contains("Env: \n").eval(&output));
        }

        #[test]
        fn sets_env_var_only() {
            let sandbox = deno_sandbox();

            sandbox.create_file("base/input1.ts", "");
            sandbox.create_file("base/input2.ts", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("deno:affectedFilesEnvVar")
                    .arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: \n").eval(&output));
            assert!(predicate::str::contains("Env: input1.ts,input2.ts\n").eval(&output));
        }
    }
}
