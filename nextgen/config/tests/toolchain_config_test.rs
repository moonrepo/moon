mod utils;

use moon_config2::{FilePath, ToolchainConfig};
use proto::ToolsConfig;
// use std::env;
use utils::*;

const FILENAME: &str = ".moon/toolchain.yml";

mod toolchain_config {
    use super::*;

    #[test]
    #[should_panic(
        expected = "unknown field `unknown`, expected one of `$schema`, `extends`, `deno`, `node`, `rust`, `typescript`"
    )]
    fn error_unknown_field() {
        test_load_config(FILENAME, "unknown: 123", |path| {
            ToolchainConfig::load_from(path, &ToolsConfig::default())
        });
    }

    #[test]
    fn loads_defaults() {
        let config = test_load_config(FILENAME, "{}", |path| {
            ToolchainConfig::load_from(path, &ToolsConfig::default())
        });

        assert!(config.deno.is_none());
        assert!(config.node.is_none());
        assert!(config.rust.is_none());
        assert!(config.typescript.is_none());
    }

    mod deno {
        use super::*;

        #[test]
        fn uses_defaults() {
            let config = test_load_config(FILENAME, "deno: {}", |path| {
                ToolchainConfig::load_from(path, &ToolsConfig::default())
            });

            let cfg = config.deno.unwrap();

            assert_eq!(cfg.deps_file, FilePath("deps.ts".to_owned()));
            assert!(!cfg.lockfile);
        }

        #[test]
        fn sets_values() {
            let config = test_load_config(
                FILENAME,
                r"
deno:
  depsFile: dependencies.ts
  lockfile: true
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );

            let cfg = config.deno.unwrap();

            assert_eq!(cfg.deps_file, FilePath("dependencies.ts".to_owned()));
            assert!(cfg.lockfile);
        }

        #[test]
        fn enables_via_proto() {
            let config = test_load_config(FILENAME, "{}", |path| {
                let mut proto = ToolsConfig::default();
                proto.tools.insert("deno".into(), "1.30.0".into());

                ToolchainConfig::load_from(path, &proto)
            });

            assert!(config.deno.is_some());
            // assert_eq!(config.deno.unwrap().version.unwrap(), "1.30.0");
        }
    }

    mod rust {
        use super::*;

        #[test]
        fn uses_defaults() {
            let config = test_load_config(FILENAME, "rust: {}", |path| {
                ToolchainConfig::load_from(path, &ToolsConfig::default())
            });

            let cfg = config.rust.unwrap();

            assert_eq!(cfg.bins, Vec::<String>::new());
            assert!(!cfg.sync_toolchain_config);
        }

        #[test]
        fn sets_values() {
            let config = test_load_config(
                FILENAME,
                r"
rust:
  bins: [cargo-make]
  syncToolchainConfig: true
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );

            let cfg = config.rust.unwrap();

            assert_eq!(cfg.bins, vec!["cargo-make"]);
            assert!(cfg.sync_toolchain_config);
        }

        #[test]
        fn enables_via_proto() {
            let config = test_load_config(FILENAME, "{}", |path| {
                let mut proto = ToolsConfig::default();
                proto.tools.insert("rust".into(), "1.69.0".into());

                ToolchainConfig::load_from(path, &proto)
            });

            assert!(config.rust.is_some());
            assert_eq!(config.rust.unwrap().version.unwrap(), "1.69.0");
        }

        #[test]
        fn proto_version_doesnt_override() {
            let config = test_load_config(
                FILENAME,
                r"
rust:
  version: 1.60.0
",
                |path| {
                    let mut proto = ToolsConfig::default();
                    proto.tools.insert("rust".into(), "1.69.0".into());

                    ToolchainConfig::load_from(path, &proto)
                },
            );

            assert!(config.rust.is_some());
            assert_eq!(config.rust.unwrap().version.unwrap(), "1.60.0");
        }

        #[test]
        #[should_panic(expected = "not a valid semantic version")]
        fn validates_version() {
            test_load_config(
                FILENAME,
                r"
rust:
  version: '1'
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );
        }

        //         #[test]
        //         fn inherits_version_from_env_var() {
        //             env::set_var("MOON_RUST_VERSION", "1.70.0");

        //             let config = test_load_config(
        //                 FILENAME,
        //                 r"
        // rust:
        //   version: 1.60.0
        // ",
        //                 |path| {
        //                     let mut proto = ToolsConfig::default();
        //                     proto.tools.insert("rust".into(), "1.65.0".into());

        //                     ToolchainConfig::load_from(path, &proto)
        //                 },
        //             );

        //             env::remove_var("MOON_RUST_VERSION");

        //             assert_eq!(config.rust.unwrap().version.unwrap(), "1.70.0");
        //         }
    }

    mod typescript {
        use super::*;

        #[test]
        fn uses_defaults() {
            let config = test_load_config(FILENAME, "typescript: {}", |path| {
                ToolchainConfig::load_from(path, &ToolsConfig::default())
            });

            let cfg = config.typescript.unwrap();

            assert_eq!(
                cfg.project_config_file_name,
                FilePath("tsconfig.json".to_owned())
            );
            assert!(cfg.sync_project_references);
        }

        #[test]
        fn sets_values() {
            let config = test_load_config(
                FILENAME,
                r"
typescript:
  projectConfigFileName: tsconf.json
  syncProjectReferences: false
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );

            let cfg = config.typescript.unwrap();

            assert_eq!(
                cfg.project_config_file_name,
                FilePath("tsconf.json".to_owned())
            );
            assert!(!cfg.sync_project_references);
        }

        #[test]
        fn enables_via_proto() {
            let config = test_load_config(FILENAME, "{}", |path| {
                let mut proto = ToolsConfig::default();
                proto.tools.insert("typescript".into(), "5.0.0".into());

                ToolchainConfig::load_from(path, &proto)
            });

            assert!(config.typescript.is_some());
            // assert_eq!(config.typescript.unwrap().version.unwrap(), "1.30.0");
        }
    }
}
