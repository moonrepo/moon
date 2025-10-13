mod utils;

use httpmock::prelude::*;
use moon_config::{ConfigLoader, ToolchainConfig, ToolchainPluginConfig};
use proto_core::{
    Id, PluginLocator, ProtoConfig, ToolContext, UnresolvedVersionSpec, warpgate::FileLocator,
};
use schematic::ConfigLoader as BaseLoader;
use serde_json::Value;
use serial_test::serial;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::path::Path;
use utils::*;

const FILENAME: &str = ".moon/toolchain.yml";

fn load_config_from_file(path: &Path) -> ToolchainConfig {
    BaseLoader::<ToolchainConfig>::new()
        .file(path)
        .unwrap()
        .load()
        .unwrap()
        .config
}

fn load_config_from_root(root: &Path, proto: &ProtoConfig) -> miette::Result<ToolchainConfig> {
    ConfigLoader::default().load_toolchain_config(root, proto)
}

mod toolchain_config {
    use super::*;

    #[test]
    fn loads_defaults() {
        let config = test_load_config(FILENAME, "{}", |path| {
            load_config_from_root(path, &ProtoConfig::default())
        });

        // system
        assert_eq!(config.plugins.len(), 1);
    }

    mod extends {
        use super::*;

        const SHARED_TOOLCHAIN: &str = r"
bun: {}
node: {}";

        #[test]
        fn recursive_merges() {
            let sandbox = create_sandbox("extends/toolchain");
            let config = test_config(sandbox.path().join("base-2.yml"), |path| {
                Ok(load_config_from_file(path))
            });

            let node = config.get_plugin_config("node").unwrap();

            assert_eq!(
                node.version.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("4.5.6").unwrap()
            );
            assert_eq!(
                node.config.get("addEnginesConstraint").unwrap(),
                &Value::Bool(true)
            );
            assert_eq!(
                node.config.get("packageManager").unwrap(),
                &Value::String("yarn".into())
            );
            assert_eq!(
                node.config.get("dedupeOnLockfileChange").unwrap(),
                &Value::Bool(false)
            );

            let yarn = config.get_plugin_config("yarn").unwrap();

            assert_eq!(
                yarn.version.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("3.3.0").unwrap()
            );
        }

        #[test]
        fn recursive_merges_typescript() {
            let sandbox = create_sandbox("extends/toolchain");
            let config = test_config(sandbox.path().join("typescript-2.yml"), |path| {
                Ok(load_config_from_file(path))
            });

            let cfg = config.get_plugin_config("typescript").unwrap();

            assert_eq!(
                cfg.config.get("rootConfigFileName").unwrap(),
                &Value::String("tsconfig.root.json".to_owned())
            );
            assert_eq!(
                cfg.config.get("createMissingConfig").unwrap(),
                &Value::Bool(false)
            );
            assert_eq!(
                cfg.config.get("syncProjectReferences").unwrap(),
                &Value::Bool(true)
            );
        }

        #[test]
        fn loads_from_url() {
            let sandbox = create_empty_sandbox();
            let server = MockServer::start();

            server.mock(|when, then| {
                when.method(GET).path("/config.yml");
                then.status(200).body(SHARED_TOOLCHAIN);
            });

            let url = server.url("/config.yml");

            sandbox.create_file(
                ".moon/toolchain.yml",
                format!(
                    r"
extends: '{url}'

deno: {{}}
"
                ),
            );

            let config = test_config(sandbox.path(), |root| {
                load_config_from_root(root, &ProtoConfig::default())
            });

            dbg!(&config);

            assert!(config.get_plugin_config("bun").is_some());
            assert!(config.get_plugin_config("deno").is_some());
            assert!(config.get_plugin_config("node").is_some());
        }

        #[test]
        fn loads_from_url_and_saves_temp_file() {
            let sandbox = create_empty_sandbox();
            let server = MockServer::start();

            server.mock(|when, then| {
                when.method(GET).path("/config.yml");
                then.status(200).body(SHARED_TOOLCHAIN);
            });

            let temp_dir = sandbox.path().join(".moon/cache/temp");
            let url = server.url("/config.yml");

            sandbox.create_file(".moon/toolchain.yml", format!(r"extends: '{url}'"));

            assert!(!temp_dir.exists());

            test_config(sandbox.path(), |root| {
                load_config_from_root(root, &ProtoConfig::default())
            });

            assert!(temp_dir.exists());
        }
    }

    mod plugin {
        use super::*;

        #[test]
        fn uses_defaults() {
            let config = test_load_config(
                FILENAME,
                r"
plugin:
  plugin: file://example.wasm
",
                |path| load_config_from_root(path, &ProtoConfig::default()),
            );

            let cfg = config.plugins.get("plugin").unwrap();

            assert_eq!(
                cfg,
                &ToolchainPluginConfig {
                    plugin: Some(PluginLocator::File(Box::new(FileLocator {
                        file: "file://example.wasm".into(),
                        path: None
                    }))),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn inherits_proto_version() {
            let config = test_load_config(
                FILENAME,
                r"
plugin:
  plugin: file://example.wasm
",
                |path| {
                    let mut proto = ProtoConfig::default();
                    proto.versions.insert(
                        ToolContext::new(Id::raw("plugin")),
                        UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                    );

                    load_config_from_root(path, &proto)
                },
            );

            let cfg = config.plugins.get("plugin").unwrap();

            assert_eq!(
                cfg.version.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("1.0.0").unwrap()
            );
        }

        #[test]
        fn inherits_proto_version_with_different_id() {
            let config = test_load_config(
                FILENAME,
                r"
plugin:
  plugin: file://example.wasm
  versionFromPrototools: 'plugin-other'
",
                |path| {
                    let mut proto = ProtoConfig::default();
                    proto.versions.insert(
                        ToolContext::new(Id::raw("plugin-other")),
                        UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                    );

                    load_config_from_root(path, &proto)
                },
            );

            let cfg = config.plugins.get("plugin").unwrap();

            assert_eq!(
                cfg.version.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("1.0.0").unwrap()
            );
        }

        #[test]
        fn doesnt_inherit_proto_version_when_disabled() {
            let config = test_load_config(
                FILENAME,
                r"
plugin:
  plugin: file://example.wasm
  versionFromPrototools: false
",
                |path| {
                    let mut proto = ProtoConfig::default();
                    proto.versions.insert(
                        ToolContext::new(Id::raw("plugin")),
                        UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
                    );

                    load_config_from_root(path, &proto)
                },
            );

            let cfg = config.plugins.get("plugin").unwrap();

            assert!(cfg.version.is_none());
        }

        #[test]
        #[serial]
        fn proto_version_doesnt_override() {
            let config = test_load_config(
                FILENAME,
                r"
plugin:
  plugin: file://example.wasm
  version: 1.0.0
",
                |path| {
                    let mut proto = ProtoConfig::default();
                    proto.versions.insert(
                        ToolContext::new(Id::raw("plugin")),
                        UnresolvedVersionSpec::parse("2.0.0").unwrap().into(),
                    );

                    load_config_from_root(path, &proto)
                },
            );

            let cfg = config.plugins.get("plugin").unwrap();

            assert_eq!(
                cfg.version.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("1.0.0").unwrap()
            );
        }
    }

    mod pkl {
        use super::*;
        use starbase_sandbox::locate_fixture;
        use std::collections::BTreeMap;

        #[test]
        fn loads_pkl() {
            let config = test_config(locate_fixture("pkl"), |path| {
                let proto = proto_core::ProtoConfig::default();
                ConfigLoader::default().load_toolchain_config(path, &proto)
            });

            assert_eq!(
                config.plugins.get("typescript").unwrap().config,
                BTreeMap::from_iter([
                    ("createMissingConfig".into(), Value::Bool(false)),
                    ("includeProjectReferenceSources".into(), Value::Bool(true)),
                    ("includeSharedTypes".into(), Value::Bool(true)),
                    (
                        "projectConfigFileName".into(),
                        Value::String("tsconfig.app.json".into())
                    ),
                    (
                        "rootConfigFileName".into(),
                        Value::String("tsconfig.root.json".into())
                    ),
                    (
                        "rootOptionsConfigFileName".into(),
                        Value::String("tsconfig.opts.json".into())
                    ),
                    ("routeOutDirToCache".into(), Value::Bool(true)),
                    ("syncProjectReferences".into(), Value::Bool(false)),
                    ("syncProjectReferencesToPaths".into(), Value::Bool(true)),
                ])
            );
        }
    }
}
