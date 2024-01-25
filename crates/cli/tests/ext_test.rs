use moon_config::PartialExtensionConfig;
use moon_test_utils::{create_sandbox_with_config, create_sandbox_with_factory, predicates};
use proto_core::{Id, PluginLocator};
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
                        plugin: Some(PluginLocator::SourceFile {
                            file: "invalid.wasm".into(),
                            path: PathBuf::from("invalid.wasm"),
                        }),
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
                "Cannot load example plugin, source file invalid.wasm does not exist.",
            ));
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
