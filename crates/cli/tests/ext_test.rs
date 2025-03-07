use moon_common::Id;
use moon_config::PartialExtensionConfig;
use moon_test_utils::{create_sandbox_with_config, create_sandbox_with_factory, predicates};
use proto_core::{PluginLocator, warpgate::FileLocator};
use rustc_hash::FxHashMap;
use std::path::PathBuf;

mod ext {
    use super::*;

    #[test]
    fn errors_if_unknown_id() {
        let sandbox = create_sandbox_with_config("base", None, None, None);

        sandbox
            .run_moon(|cmd| {
                cmd.arg("ext").arg("unknown");
            })
            .failure()
            .stderr(predicates::str::contains(
                "The extension unknown does not exist.",
            ));
    }
}

mod ext_download {
    use super::*;

    #[test]
    fn errors_if_no_args() {
        let sandbox = create_sandbox_with_config("base", None, None, None);

        sandbox
            .run_moon(|cmd| {
                cmd.arg("ext").arg("download");
            })
            .failure()
            .stderr(predicates::str::contains(
                "the following required arguments were not provided",
            ));
    }

    #[test]
    fn errors_if_no_plugin_locator() {
        let sandbox = create_sandbox_with_factory("base", |workspace, _, _| {
            workspace
                .extensions
                .get_or_insert(FxHashMap::default())
                .insert(
                    Id::raw("example"),
                    PartialExtensionConfig {
                        plugin: None,
                        config: None,
                    },
                );
        });

        sandbox
            .run_moon(|cmd| {
                cmd.arg("ext").arg("example");
            })
            .failure()
            .stderr(predicates::str::contains(
                "extensions.example.plugin: this setting is required",
            ));
    }

    #[test]
    fn errors_if_invalid_plugin_locator() {
        let sandbox = create_sandbox_with_factory("base", |workspace, _, _| {
            workspace
                .extensions
                .get_or_insert(FxHashMap::default())
                .insert(
                    Id::raw("example"),
                    PartialExtensionConfig {
                        plugin: Some(PluginLocator::File(Box::new(FileLocator {
                            file: "invalid.wasm".into(),
                            path: Some(PathBuf::from("invalid.wasm")),
                        }))),
                        config: None,
                    },
                );
        });

        sandbox
            .run_moon(|cmd| {
                cmd.arg("ext").arg("example");
            })
            .failure()
            .stderr(predicates::str::contains("Cannot load example plugin"));
    }

    #[test]
    fn executes_the_plugin() {
        let sandbox = create_sandbox_with_config("base", None, None, None);

        sandbox
            .run_moon(|cmd| {
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
        let sandbox = create_sandbox_with_config("base", None, None, None);
        sandbox.create_file("nx.json", "{}");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("ext").arg("migrate-nx").arg("--").arg("--cleanup");
        });

        assert
            .success()
            .stdout(predicates::str::contains("Successfully migrated from Nx"));

        assert!(!sandbox.path().join("nx.json").exists());
    }
}

mod ext_migrate_turborepo {
    use super::*;

    #[test]
    fn executes_the_plugin() {
        let sandbox = create_sandbox_with_config("base", None, None, None);
        sandbox.create_file("turbo.json", "{}");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("ext")
                .arg("migrate-turborepo")
                .arg("--")
                .arg("--cleanup");
        });

        assert.success().stdout(predicates::str::contains(
            "Successfully migrated from Turborepo",
        ));

        assert!(!sandbox.path().join("turbo.json").exists());
    }
}
