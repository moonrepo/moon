use moon_test_utils2::{create_empty_moon_sandbox, create_moon_sandbox, predicates::prelude::*};
use proto_core::VersionReq;
use std::fs;

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

    // .config/moon
    mod config_dir {
        use super::*;

        #[test]
        fn supports() {
            let sandbox = create_moon_sandbox("projects");

            fs::create_dir_all(sandbox.path().join(".config")).unwrap();

            fs::rename(
                sandbox.path().join(".moon"),
                sandbox.path().join(".config/moon"),
            )
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("sync");
                })
                .success();
        }

        #[test]
        #[should_panic(expected = "Unable to determine workspace root")]
        fn errors_if_not_found() {
            let sandbox = create_moon_sandbox("projects");

            fs::create_dir_all(sandbox.path().join(".config")).unwrap();

            fs::rename(
                sandbox.path().join(".moon"),
                sandbox.path().join(".config/moon-invalid"),
            )
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("sync");
                })
                .failure()
                .stderr(predicate::str::contains(">=1000.0.0"));
        }
    }
}
