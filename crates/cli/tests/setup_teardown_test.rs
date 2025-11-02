use moon_common::Id;
use moon_config::PartialToolchainPluginConfig;
use moon_test_utils2::{create_empty_moon_sandbox, predicates::prelude::*};
use proto_core::UnresolvedVersionSpec;
use starbase_utils::dirs;

mod setup {
    use super::*;

    #[test]
    fn does_nothing_if_no_toolchains() {
        let sandbox = create_empty_moon_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("setup");
            })
            .success()
            .stdout(predicate::str::contains("no toolchains are configured"));
    }
}

mod setup_teardown {
    use super::*;

    #[test]
    fn sets_up_and_tears_down() {
        let sandbox = create_empty_moon_sandbox();

        sandbox.update_toolchains_config(|config| {
            config.plugins.get_or_insert_default().insert(
                Id::raw("node"),
                PartialToolchainPluginConfig {
                    version: Some(UnresolvedVersionSpec::parse("21.0.0").unwrap()),
                    ..Default::default()
                },
            );
        });

        let proto_dir = dirs::home_dir().unwrap().join(".proto");
        let node_dir = proto_dir.join("tools/node/21.0.0");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("setup");
            })
            .success()
            .code(1);

        assert!(node_dir.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("teardown");
            })
            .success()
            .code(0);

        assert!(!node_dir.exists());
    }
}
