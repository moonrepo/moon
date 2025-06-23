use moon_test_utils2::create_empty_moon_sandbox;
use starbase_sandbox::{assert_snapshot, predicates::prelude::*};

mod toolchain_info {
    use super::*;

    #[test]
    fn renders() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("toolchain").arg("info").arg("typescript");
        });

        assert_snapshot!(assert.output_standardized());

        assert.success();
    }

    #[test]
    fn errors_for_missing_locator() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("toolchain").arg("info").arg("unknown");
            })
            .failure();

        assert.stderr(predicate::str::contains(
            "A plugin locator string is required",
        ));
    }
}
