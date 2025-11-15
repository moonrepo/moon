use moon_common::Id;
use moon_config::PartialExtensionPluginConfig;
use moon_test_utils2::{create_moon_sandbox, predicates};
use proto_core::{PluginLocator, warpgate::FileLocator};
use std::path::PathBuf;

mod ext {
    use super::*;

    #[test]
    fn errors_if_unknown_id() {
        let sandbox = create_moon_sandbox("extensions");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("ext").arg("unknown");
            })
            .failure()
            .stderr(predicates::str::contains(
                "The extension plugin unknown does not exist",
            ));
    }
}

mod ext_download {
    use super::*;

    #[test]
    fn errors_if_no_args() {
        let sandbox = create_moon_sandbox("extensions");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("ext").arg("download");
            })
            .failure()
            .stderr(predicates::str::contains(
                "the following required arguments were not provided",
            ));
    }

    #[test]
    fn errors_if_no_plugin_locator() {
        let sandbox = create_moon_sandbox("extensions");

        sandbox.update_extensions_config(|config| {
            config
                .plugins
                .get_or_insert_default()
                .insert(Id::raw("example"), PartialExtensionPluginConfig::default());
        });

        sandbox
            .run_bin(|cmd| {
                cmd.arg("ext").arg("example");
            })
            .failure()
            .stderr(predicates::str::contains(
                "example.plugin: a locator is required for plugins",
            ));
    }

    #[test]
    fn errors_if_invalid_plugin_locator() {
        let sandbox = create_moon_sandbox("extensions");

        sandbox.update_extensions_config(|config| {
            config.plugins.get_or_insert_default().insert(
                Id::raw("example"),
                PartialExtensionPluginConfig {
                    plugin: Some(PluginLocator::File(Box::new(FileLocator {
                        file: "invalid.wasm".into(),
                        path: Some(PathBuf::from("invalid.wasm")),
                    }))),
                    ..Default::default()
                },
            );
        });

        sandbox
            .run_bin(|cmd| {
                cmd.arg("ext").arg("example");
            })
            .failure()
            .stderr(predicates::str::contains("Cannot load example plugin"));
    }

    #[test]
    fn executes_the_plugin() {
        let sandbox = create_moon_sandbox("extensions");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("ext").arg("download").args([
                    "--",
                    "--url",
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md",
                ]);
            })
            .success()
            .stdout(predicates::str::contains("Downloaded to"));
    }
}

mod ext_migrate_nx {
    use super::*;

    #[test]
    fn executes_the_plugin() {
        let sandbox = create_moon_sandbox("extensions");
        sandbox.create_file("nx.json", "{}");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("ext").arg("migrate-nx").arg("--").arg("--cleanup");
            })
            .success()
            .stdout(predicates::str::contains("Successfully migrated from Nx"));

        assert!(!sandbox.path().join("nx.json").exists());
    }
}

mod ext_migrate_turborepo {
    use super::*;

    #[test]
    fn executes_the_plugin() {
        let sandbox = create_moon_sandbox("extensions");
        sandbox.create_file("turbo.json", "{}");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("ext")
                    .arg("migrate-turborepo")
                    .arg("--")
                    .arg("--cleanup");
            })
            .success()
            .stdout(predicates::str::contains(
                "Successfully migrated from Turborepo",
            ));

        assert!(!sandbox.path().join("turbo.json").exists());
    }
}
