#![allow(deprecated)]

mod utils;

use httpmock::prelude::*;
use moon_config::{
    BinConfig, BinEntry, ConfigLoader, NodePackageManager, NodeVersionFormat, ToolchainConfig,
    ToolchainPluginConfig,
};
use proto_core::{
    Id, PluginLocator, ProtoConfig, ToolContext, UnresolvedVersionSpec, warpgate::FileLocator,
};
use schematic::ConfigLoader as BaseLoader;
use serde_json::Value;
use serial_test::serial;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::env;
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

        assert!(config.plugins.is_empty());
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

            let node = config.node.unwrap();

            assert_eq!(
                node.version.unwrap(),
                UnresolvedVersionSpec::parse("4.5.6").unwrap()
            );
            assert!(node.add_engines_constraint);
            assert!(!node.dedupe_on_lockfile_change);
            assert_eq!(node.package_manager, NodePackageManager::Yarn);

            let yarn = node.yarn.unwrap();

            assert_eq!(
                yarn.version.unwrap(),
                UnresolvedVersionSpec::parse("3.3.0").unwrap()
            );
        }

        #[test]
        fn recursive_merges_typescript() {
            let sandbox = create_sandbox("extends/toolchain");
            let config = test_config(sandbox.path().join("typescript-2.yml"), |path| {
                Ok(load_config_from_file(path))
            });

            let cfg = config.plugins.get("typescript").unwrap();

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

            assert!(config.bun.is_some());
            assert!(config.deno.is_some());
            assert!(config.node.is_some());
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

    mod bun {
        use super::*;

        // #[test]
        // fn uses_defaults() {
        //     let config = test_load_config(FILENAME, "bun: {}", |path| {
        //         load_config_from_root(path, &ProtoConfig::default())
        //     });

        //     let cfg = config.bun.unwrap();

        //     assert!(cfg.plugin.is_some());
        // }

        // #[test]
        // fn enables_via_proto() {
        //     let config = test_load_config(FILENAME, "{}", |path| {
        //         let mut proto = ProtoConfig::default();
        //         proto.versions.insert(
        //             ToolContext::new(Id::raw("bun")),
        //             UnresolvedVersionSpec::parse("1.0.0").unwrap().into(),
        //         );

        //         load_config_from_root(path, &proto)
        //     });

        //     assert!(config.bun.is_some());
        //     assert_eq!(
        //         config.bun.unwrap().version.unwrap(),
        //         UnresolvedVersionSpec::parse("1.0.0").unwrap()
        //     );
        // }

        #[test]
        fn inherits_plugin_locator() {
            let config = test_load_config(FILENAME, "bun: {}", |path| {
                let mut tools = ProtoConfig::default();
                tools.inherit_builtin_plugins();

                load_config_from_root(path, &tools)
            });

            assert!(config.bun.unwrap().plugin.is_some());
        }

        #[test]
        #[serial]
        fn proto_version_doesnt_override() {
            let config = test_load_config(
                FILENAME,
                r"
bun:
  version: 1.0.0
",
                |path| {
                    let mut proto = ProtoConfig::default();
                    proto.versions.insert(
                        ToolContext::new(Id::raw("bun")),
                        UnresolvedVersionSpec::parse("2.0.0").unwrap().into(),
                    );

                    load_config_from_root(path, &proto)
                },
            );

            assert!(config.bun.is_some());
            assert_eq!(
                config.bun.unwrap().version.unwrap(),
                UnresolvedVersionSpec::parse("1.0.0").unwrap()
            );
        }

        #[test]
        #[serial]
        fn inherits_version_from_env_var() {
            unsafe { env::set_var("MOON_BUN_VERSION", "1.0.0") };

            let config = test_load_config(
                FILENAME,
                r"
bun:
  version: 3.0.0
",
                |path| {
                    let mut proto = ProtoConfig::default();
                    proto.versions.insert(
                        ToolContext::new(Id::raw("bun")),
                        UnresolvedVersionSpec::parse("2.0.0").unwrap().into(),
                    );

                    load_config_from_root(path, &proto)
                },
            );

            unsafe { env::remove_var("MOON_BUN_VERSION") };

            assert_eq!(
                config.bun.unwrap().version.unwrap(),
                UnresolvedVersionSpec::parse("1.0.0").unwrap()
            );
        }

        #[test]
        fn inherits_version_from_node_pm() {
            let config = test_load_config(
                FILENAME,
                r"
bun: {}
node:
  packageManager: bun
  bun:
    version: 1.0.0
",
                |path| load_config_from_root(path, &ProtoConfig::default()),
            );

            assert_eq!(
                config.bun.unwrap().version.unwrap(),
                UnresolvedVersionSpec::parse("1.0.0").unwrap()
            );

            assert_eq!(
                config.node.unwrap().bun.unwrap().version.unwrap(),
                UnresolvedVersionSpec::parse("1.0.0").unwrap()
            );
        }

        #[test]
        fn inherits_args_from_node_pm() {
            let config = test_load_config(
                FILENAME,
                r"
bun: {}
node:
  packageManager: bun
  bun:
    version: 1.0.0
    installArgs: [--frozen]
",
                |path| load_config_from_root(path, &ProtoConfig::default()),
            );

            assert_eq!(config.bun.unwrap().install_args, vec!["--frozen"]);

            assert_eq!(
                config.node.unwrap().bun.unwrap().install_args,
                vec!["--frozen"]
            );
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
        use moon_config::*;
        use starbase_sandbox::locate_fixture;
        use std::collections::BTreeMap;

        #[test]
        fn loads_pkl() {
            let mut config = test_config(locate_fixture("pkl"), |path| {
                let proto = proto_core::ProtoConfig::default();
                ConfigLoader::default().load_toolchain_config(path, &proto)
            });

            assert_eq!(
                config.node.take().unwrap(),
                NodeConfig {
                    add_engines_constraint: false,
                    bin_exec_args: vec!["--profile".into()],
                    bun: None,
                    dedupe_on_lockfile_change: false,
                    dependency_version_format: NodeVersionFormat::WorkspaceCaret,
                    infer_tasks_from_scripts: true,
                    npm: NpmConfig::default(),
                    package_manager: NodePackageManager::Yarn,
                    packages_root: ".".into(),
                    plugin: None,
                    pnpm: None,
                    root_package_only: true,
                    sync_package_manager_field: false,
                    sync_project_workspace_dependencies: false,
                    sync_version_manager_config: Some(NodeVersionManager::Nvm),
                    version: Some(UnresolvedVersionSpec::parse("20.12").unwrap()),
                    yarn: Some(YarnConfig {
                        install_args: vec!["--immutable".into()],
                        plugin: None,
                        plugins: vec![],
                        version: Some(UnresolvedVersionSpec::parse("4").unwrap())
                    })
                }
            );
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
