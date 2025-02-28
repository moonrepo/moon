use moon_common::Id;
use moon_config::{
    PartialRustConfig, PartialToolchainConfig, PartialWorkspaceConfig, PartialWorkspaceProjects,
};
use moon_test_utils::{
    Sandbox, assert_snapshot, create_sandbox_with_config, predicates::prelude::*,
};
use rustc_hash::FxHashMap;

fn rust_sandbox() -> Sandbox {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([(
            Id::raw("rust"),
            ".".into(),
        )]))),
        ..PartialWorkspaceConfig::default()
    };

    let toolchain_config = PartialToolchainConfig {
        rust: Some(PartialRustConfig::default()),
        ..PartialToolchainConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "rust/cases",
        Some(workspace_config),
        Some(toolchain_config),
        None,
    );
    sandbox.enable_git();
    sandbox
}

#[test]
fn runs_standard_script() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:standard");
    });

    let output = assert.output();

    assert!(predicate::str::contains("stderr").eval(&output));
    assert!(predicate::str::contains("stdout").eval(&output));
}

#[test]
fn handles_process_exit_zero() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:exitZero");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn handles_process_exit_nonzero() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:exitNonZero");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn handles_panic() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:panic");
    });

    let output = assert.output();

    assert!(
        predicate::str::is_match("thread 'main' panicked at(?s:.)*Oops")
            .unwrap()
            .eval(&output)
    );
}

#[test]
fn sets_env_vars() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:envVars");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn inherits_moon_env_vars() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:envVarsMoon");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn forces_cache_to_write_only() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:envVarsMoon").arg("--updateCache");
    });

    assert!(predicate::str::contains("MOON_CACHE=write").eval(&assert.output()));
}

#[test]
fn runs_from_project_root() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:runFromProject");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn runs_from_workspace_root() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:runFromWorkspace");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn retries_on_failure_till_count() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:retryCount");
    });

    let output = assert.output();

    assert!(predicate::str::contains("exit code 1").eval(&output));
}

#[test]
fn runs_script_task() {
    let sandbox = rust_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("rust:viaScript");
    });

    let output = assert.output();

    // Versions aren't pinned, so don't use snapshots
    assert!(predicate::str::contains("rust platform").eval(&output));
}

mod rustup_toolchain {
    use super::*;

    fn rust_toolchain_sandbox() -> Sandbox {
        let workspace_config = PartialWorkspaceConfig {
            projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([(
                Id::raw("rust"),
                ".".into(),
            )]))),
            ..PartialWorkspaceConfig::default()
        };

        let toolchain_config = PartialToolchainConfig {
            rust: Some(PartialRustConfig {
                components: Some(vec!["clippy".into()]),
                targets: Some(vec!["wasm32-wasip1".into()]),
                ..PartialRustConfig::default()
            }),
            ..PartialToolchainConfig::default()
        };

        let sandbox = create_sandbox_with_config(
            "rust/toolchain",
            Some(workspace_config),
            Some(toolchain_config),
            None,
        );
        sandbox.enable_git();
        sandbox
    }

    #[test]
    fn installs_components_and_targets() {
        let sandbox = rust_toolchain_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("rust:noop");
        });

        let output = assert.output();

        assert!(predicate::str::contains("rustup component").eval(&output));
        assert!(predicate::str::contains("rustup target").eval(&output));
    }

    #[test]
    fn doesnt_install_again() {
        let sandbox = rust_toolchain_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("rust:noop");
        });

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("rust:noop");
        });

        let output = assert.output();

        assert!(
            predicate::str::contains("rustup component")
                .not()
                .eval(&output)
        );
        assert!(
            predicate::str::contains("rustup target")
                .not()
                .eval(&output)
        );
    }
}
