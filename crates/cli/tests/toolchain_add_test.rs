use moon_test_utils::create_empty_moon_sandbox;
use starbase_sandbox::{assert_snapshot, predicates::prelude::*};
use std::fs;

mod toolchain_add {
    use super::*;

    #[test]
    fn errors_for_missing_locator() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("toolchain").arg("add").arg("tc").arg("--yes");
            })
            .failure();

        assert.stderr(predicate::str::contains(
            "A plugin locator string is required",
        ));
    }

    #[test]
    fn errors_for_invalid_locator() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("toolchain")
                    .arg("add")
                    .arg("tc")
                    .arg("invalid")
                    .arg("--yes");
            })
            .failure();

        assert.stderr(predicate::str::contains("Missing plugin protocol"));
    }

    #[test]
    fn renders_full() {
        let sandbox = create_empty_moon_sandbox();

        sandbox.run_bin(|cmd| {
            cmd.arg("toolchain")
                .arg("add")
                .arg("typescript")
                .arg("--yes");
        });

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join(".moon").join("toolchains.yml")).unwrap()
        );
    }

    #[test]
    fn renders_minimal() {
        let sandbox = create_empty_moon_sandbox();

        sandbox.run_bin(|cmd| {
            cmd.arg("toolchain")
                .arg("add")
                .arg("typescript")
                .arg("--yes")
                .arg("--minimal");
        });

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join(".moon").join("toolchains.yml")).unwrap()
        );
    }

    // `tc-tier3-tool` inherits the root of `tc-tier2` and requires
    // `tc-tier2-reqs`, which requires `tc-tier3`. Each is added before the
    // toolchain that needs it, and a select prompt that has the toolchain
    // it's added for as an option (`testAddedFor`) defaults to it
    #[test]
    fn adds_the_toolchains_it_needs_first() {
        let sandbox = create_empty_moon_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("toolchain")
                    .arg("add")
                    .arg("tc-tier3-tool")
                    .arg("--yes");
            })
            .success()
            .stdout(predicate::str::contains(
                "Added toolchain tc-tier2 (needed by tc-tier3-tool)",
            ))
            .stdout(predicate::str::contains(
                "Added toolchain tc-tier3 (needed by tc-tier2-reqs)",
            ));

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join(".moon").join("toolchains.yml")).unwrap()
        );
    }

    #[test]
    fn doesnt_add_a_needed_toolchain_that_is_configured() {
        let sandbox = create_empty_moon_sandbox();

        sandbox.create_file(".moon/toolchains.yml", "tc-tier2: {}\n");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("toolchain")
                    .arg("add")
                    .arg("tc-tier3-tool")
                    .arg("--yes");
            })
            .success();

        let config =
            fs::read_to_string(sandbox.path().join(".moon").join("toolchains.yml")).unwrap();

        assert_eq!(config.matches("tc-tier2:").count(), 1, "{config}");
        assert!(config.contains("tc-tier2-reqs:"), "{config}");
        assert!(config.contains("tc-tier3:"), "{config}");
        assert!(config.contains("tc-tier3-tool:"), "{config}");
    }
}
