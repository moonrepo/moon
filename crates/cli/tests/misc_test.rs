use moon_test_utils2::{create_empty_moon_sandbox, predicates::prelude::*};
use proto_core::VersionReq;

mod cli {
    use super::*;

    #[test]
    fn fails_on_version_constraint() {
        let sandbox = create_empty_moon_sandbox();

        sandbox.update_workspace_config(|config| {
            config.version_constraint = Some(VersionReq::parse(">=1000.0.0").unwrap());
        });

        sandbox
            .run_bin(|cmd| {
                cmd.arg("sync");
            })
            .failure()
            .stderr(predicate::str::contains(">=1000.0.0"));
    }
}
