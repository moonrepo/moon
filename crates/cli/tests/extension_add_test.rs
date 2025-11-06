use moon_test_utils2::create_empty_moon_sandbox;
use starbase_sandbox::{assert_snapshot, predicates::prelude::*};
use std::fs;

mod extension_add {
    use super::*;

    #[test]
    fn errors_for_missing_locator() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("extension").arg("add").arg("test").arg("--yes");
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
                cmd.arg("extension")
                    .arg("add")
                    .arg("test")
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
            cmd.arg("extension").arg("add").arg("download").arg("--yes");
        });

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join(".moon").join("extensions.yml")).unwrap()
        );
    }

    #[test]
    fn renders_minimal() {
        let sandbox = create_empty_moon_sandbox();

        sandbox.run_bin(|cmd| {
            cmd.arg("extension")
                .arg("add")
                .arg("download")
                .arg("--yes")
                .arg("--minimal");
        });

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join(".moon").join("extensions.yml")).unwrap()
        );
    }
}
