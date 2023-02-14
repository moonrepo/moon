use httpmock::prelude::*;
use moon_config::{ConfigError, NodeConfig, ToolchainConfig};
use moon_constants::CONFIG_TOOLCHAIN_FILENAME;
use moon_test_utils::get_fixtures_path;
use std::path::Path;

fn load_jailed_config(root: &Path) -> Result<ToolchainConfig, figment::Error> {
    match ToolchainConfig::load(root.join(CONFIG_TOOLCHAIN_FILENAME)) {
        Ok(cfg) => Ok(cfg),
        Err(err) => Err(match err {
            ConfigError::FailedValidation(errors) => errors.first().unwrap().to_owned(),
            ConfigError::Figment(f) => f,
            e => figment::Error::from(e.to_string()),
        }),
    }
}

#[test]
fn loads_defaults() {
    figment::Jail::expect_with(|jail| {
        jail.create_file(CONFIG_TOOLCHAIN_FILENAME, "{}")?;

        let config = load_jailed_config(jail.directory())?;

        assert_eq!(
            config,
            ToolchainConfig {
                extends: None,
                node: None,
                typescript: None,
                schema: String::new(),
            }
        );

        Ok(())
    });
}

mod extends {
    use super::*;
    use moon_config::{NodePackageManager, TypeScriptConfig, YarnConfig};
    use moon_test_utils::pretty_assertions::assert_eq;
    use std::fs;

    #[test]
    fn recursive_merges() {
        let fixture = get_fixtures_path("config-extends/toolchain");
        let config = ToolchainConfig::load(fixture.join("base-2.yml")).unwrap();

        assert_eq!(
            config,
            ToolchainConfig {
                node: Some(NodeConfig {
                    version: Some("4.5.6".into()),
                    add_engines_constraint: true,
                    dedupe_on_lockfile_change: false,
                    package_manager: NodePackageManager::Yarn,
                    yarn: Some(YarnConfig {
                        plugins: None,
                        version: "3.3.0".into()
                    }),
                    ..NodeConfig::default()
                }),
                ..ToolchainConfig::default()
            }
        );
    }

    #[test]
    fn recursive_merges_typescript() {
        let fixture = get_fixtures_path("config-extends/toolchain");
        let config = ToolchainConfig::load(fixture.join("typescript-2.yml")).unwrap();

        assert_eq!(
            config.typescript,
            Some(TypeScriptConfig {
                create_missing_config: false,
                root_config_file_name: "tsconfig.root.json".into(),
                sync_project_references: true,
                ..TypeScriptConfig::default()
            })
        );
    }

    #[test]
    #[should_panic(expected = "Invalid <id>extends</id> field, must be a string.")]
    // #[should_panic(
    //     expected = "invalid type: found unsigned int `123`, expected a string for key \"workspace.extends\""
    // )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_TOOLCHAIN_FILENAME, "extends: 123")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "only YAML documents are supported")]
    // #[should_panic(
    //     expected = "Must be a valid URL or relative file path (starts with ./) for key \"workspace.extends\""
    // )]
    fn not_a_url_or_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_TOOLCHAIN_FILENAME, "extends: random value")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "only HTTPS URLs are supported")]
    fn not_a_https_url() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                "extends: http://domain.com/config.yml",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "only YAML documents are supported")]
    // #[should_panic(expected = "Must be a YAML document for key \"workspace.extends\"")]
    fn not_a_yaml_url() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                "extends: https://domain.com/file.txt",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "only YAML documents are supported")]
    // #[should_panic(expected = "Must be a YAML document for key \"workspace.extends\"")]
    fn not_a_yaml_file() {
        figment::Jail::expect_with(|jail| {
            fs::create_dir_all(jail.directory().join("shared")).unwrap();

            jail.create_file("shared/file.txt", "")?;

            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                "extends: ./shared/file.txt",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    fn loads_from_file() {
        figment::Jail::expect_with(|jail| {
            fs::create_dir_all(jail.directory().join("shared")).unwrap();

            jail.create_file(
                format!("shared/{}", super::CONFIG_TOOLCHAIN_FILENAME),
                include_str!("../../../../tests/fixtures/config-extends/.moon/toolchain.yml"),
            )?;

            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
extends: ./shared/toolchain.yml

node:
    version: '18.0.0'
    npm:
        version: '8.0.0'
"#,
            )?;

            let config: ToolchainConfig = super::load_jailed_config(jail.directory())?;

            // Inherits from extended file
            assert!(!config.node.as_ref().unwrap().add_engines_constraint);
            assert!(!config.typescript.unwrap().sync_project_references);

            // Ensure we can override the extended config
            assert_eq!(
                config.node.as_ref().unwrap().version.as_ref().unwrap(),
                "18.0.0"
            );
            assert_eq!(
                config.node.as_ref().unwrap().npm.version,
                "8.0.0".to_owned()
            );

            Ok(())
        });
    }

    #[test]
    fn loads_from_url() {
        let server = MockServer::start();

        server.mock(|when, then| {
            when.method(GET).path("/config.yml");
            then.status(200).body(include_str!(
                "../../../../tests/fixtures/config-extends/.moon/toolchain.yml"
            ));
        });

        let url = server.url("/config.yml");

        figment::Jail::expect_with(|jail| {
            jail.set_env(
                "MOON_WORKSPACE_ROOT",
                jail.directory().to_owned().to_string_lossy(),
            );

            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                format!(
                    r#"
extends: '{url}'

node:
    version: '18.0.0'
    npm:
        version: '8.0.0'
"#
                )
                .as_ref(),
            )?;

            let config: ToolchainConfig = super::load_jailed_config(jail.directory())?;

            // Inherits from extended file
            assert!(!config.node.as_ref().unwrap().add_engines_constraint);
            assert!(!config.typescript.unwrap().sync_project_references);

            // Ensure we can override the extended config
            assert_eq!(
                config.node.as_ref().unwrap().version.as_ref().unwrap(),
                "18.0.0"
            );
            assert_eq!(
                config.node.as_ref().unwrap().npm.version,
                "8.0.0".to_owned()
            );

            Ok(())
        });
    }

    // #[test]
    // #[should_panic(expected = "TODO")]
    // fn handles_invalid_url() {
    //     figment::Jail::expect_with(|jail| {
    //         jail.create_file(
    //             super::CONFIG_TOOLCHAIN_FILENAME,
    //             "extends: https://raw.githubusercontent.com/this/is/an/invalid/file.yml",
    //         )?;

    //         super::load_jailed_config(jail.directory())?;

    //         Ok(())
    //     });
    // }
}

mod node {
    use super::*;
    use moon_config::NodePackageManager;

    #[test]
    fn loads_defaults() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    packageManager: yarn"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config,
                ToolchainConfig {
                    node: Some(NodeConfig {
                        package_manager: NodePackageManager::Yarn,
                        ..NodeConfig::default()
                    }),
                    ..ToolchainConfig::default()
                }
            );

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected struct NodeConfig for key \"toolchain.node\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_TOOLCHAIN_FILENAME, "node: 123")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid semantic version for key \"toolchain.node.version\""
    )]
    fn invalid_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
  version: 'foo bar'"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid semantic version for key \"toolchain.node.version\""
    )]
    fn no_patch_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
  version: '16.13'"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid semantic version for key \"toolchain.node.version\""
    )]
    fn no_minor_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
  version: '16'"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "unknown variant: found `what`, expected `one of `npm`, `pnpm`, `yarn`` for key \"toolchain.node.packageManager\""
    )]
    fn invalid_package_manager() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
  version: '16.13.0'
  packageManager: what"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    fn valid_package_manager() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
  version: '16.13.0'
  packageManager: yarn"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    fn inherits_from_env_var() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("MOON_NODE_VERSION", "4.5.6");

            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    version: '16.13.0'
"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(config.node.unwrap().version.unwrap(), String::from("4.5.6"));

            Ok(())
        });
    }
}

mod npm {
    #[test]
    #[should_panic(
        expected = "invalid type: found string \"foo\", expected struct NpmConfig for key \"toolchain.node.npm\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    version: '16.13.0'
    npm: foo"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid semantic version for key \"toolchain.node.npm.version\""
    )]
    fn invalid_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    version: '16.13.0'
    npm:
        version: 'foo bar'
"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    fn inherits_from_env_var() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("MOON_NPM_VERSION", "4.5.6");

            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    version: '16.13.0'
    npm:
        version: '1.2.3'
"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(config.node.unwrap().npm.version, String::from("4.5.6"));

            Ok(())
        });
    }
}

mod pnpm {
    #[test]
    #[should_panic(
        expected = "invalid type: found string \"foo\", expected struct PnpmConfig for key \"toolchain.node.pnpm\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    version: '16.13.0'
    pnpm: foo"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid semantic version for key \"toolchain.node.pnpm.version\""
    )]
    fn invalid_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    version: '16.13.0'
    pnpm:
        version: 'foo bar'"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    fn inherits_from_env_var() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("MOON_PNPM_VERSION", "4.5.6");

            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    version: '16.13.0'
    packageManager: 'pnpm'
    pnpm:
        version: '1.2.3'
"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config.node.unwrap().pnpm.unwrap().version,
                String::from("4.5.6")
            );

            Ok(())
        });
    }
}

mod yarn {
    #[test]
    #[should_panic(
        expected = "invalid type: found string \"foo\", expected struct YarnConfig for key \"toolchain.node.yarn\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    version: '16.13.0'
    yarn: foo"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid semantic version for key \"toolchain.node.yarn.version\""
    )]
    fn invalid_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    version: '16.13.0'
    yarn:
        version: 'foo bar'"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    fn inherits_from_env_var() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("MOON_YARN_VERSION", "4.5.6");

            jail.create_file(
                super::CONFIG_TOOLCHAIN_FILENAME,
                r#"
node:
    version: '16.13.0'
    packageManager: 'yarn'
    yarn:
        version: '1.2.3'
"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config.node.unwrap().yarn.unwrap().version,
                String::from("4.5.6")
            );

            Ok(())
        });
    }
}
