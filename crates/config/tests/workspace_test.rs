use moon_config::{
    ConfigError, GeneratorConfig, HasherConfig, NodeConfig, NotifierConfig, RunnerConfig,
    VcsConfig, VcsManager, WorkspaceConfig, WorkspaceProjects,
};
use moon_constants::CONFIG_WORKSPACE_FILENAME;
use moon_utils::test::get_fixtures_dir;
use std::path::Path;

fn load_jailed_config(root: &Path) -> Result<WorkspaceConfig, figment::Error> {
    match WorkspaceConfig::load(root.join(CONFIG_WORKSPACE_FILENAME)) {
        Ok(cfg) => Ok(cfg),
        Err(error) => Err(match error {
            ConfigError::FailedValidation(errors) => errors.first().unwrap().to_owned(),
            ConfigError::Figment(f) => f,
            e => figment::Error::from(e.to_string()),
        }),
    }
}

#[test]
fn loads_defaults() {
    figment::Jail::expect_with(|jail| {
        jail.create_file(CONFIG_WORKSPACE_FILENAME, "projects: {}")?;

        let config = load_jailed_config(jail.directory())?;

        assert_eq!(
            config,
            WorkspaceConfig {
                runner: RunnerConfig::default(),
                generator: GeneratorConfig::default(),
                extends: None,
                hasher: HasherConfig::default(),
                node: None,
                notifier: NotifierConfig::default(),
                projects: WorkspaceProjects::default(),
                typescript: None,
                vcs: VcsConfig::default(),
                schema: String::new(),
            }
        );

        Ok(())
    });
}

mod extends {
    use super::*;
    use moon_config::{NodePackageManager, TypeScriptConfig, YarnConfig};
    use pretty_assertions::assert_eq;
    use std::fs;

    #[test]
    fn recursive_merges() {
        let fixture = get_fixtures_dir("config-extends/workspace");
        let config = WorkspaceConfig::load(fixture.join("base-2.yml")).unwrap();

        assert_eq!(
            config,
            WorkspaceConfig {
                runner: RunnerConfig {
                    cache_lifetime: "3 hours".into(),
                    log_running_command: false,
                    ..RunnerConfig::default()
                },
                node: Some(NodeConfig {
                    version: "4.5.6".into(),
                    add_engines_constraint: true,
                    dedupe_on_lockfile_change: false,
                    package_manager: NodePackageManager::Yarn,
                    yarn: Some(YarnConfig {
                        plugins: None,
                        version: "3.0.0".into()
                    }),
                    ..NodeConfig::default()
                }),
                vcs: VcsConfig {
                    manager: VcsManager::Svn,
                    ..VcsConfig::default()
                },
                ..WorkspaceConfig::default()
            }
        );
    }

    #[test]
    fn recursive_merges_typescript() {
        let fixture = get_fixtures_dir("config-extends/workspace");
        let config = WorkspaceConfig::load(fixture.join("typescript-2.yml")).unwrap();

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
            jail.create_file(super::CONFIG_WORKSPACE_FILENAME, "extends: 123")?;

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
            jail.create_file(super::CONFIG_WORKSPACE_FILENAME, "extends: random value")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "only HTTPS URLs are supported")]
    fn not_a_https_url() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
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
                super::CONFIG_WORKSPACE_FILENAME,
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
                super::CONFIG_WORKSPACE_FILENAME,
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
                format!("shared/{}", super::CONFIG_WORKSPACE_FILENAME),
                include_str!("../../../tests/fixtures/config-extends/.moon/workspace.yml"),
            )?;

            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
extends: ./shared/workspace.yml

node:
    version: '18.0.0'
    npm:
        version: '8.0.0'
"#,
            )?;

            let config: WorkspaceConfig = super::load_jailed_config(jail.directory())?;

            // Inherits from extended file
            assert!(!config.node.as_ref().unwrap().add_engines_constraint);
            assert!(!config.typescript.unwrap().sync_project_references);
            assert_eq!(config.vcs.manager, VcsManager::Svn);

            // Ensure we can override the extended config
            assert_eq!(config.node.as_ref().unwrap().version, "18.0.0".to_owned());
            assert_eq!(
                config.node.as_ref().unwrap().npm.version,
                "8.0.0".to_owned()
            );

            Ok(())
        });
    }

    #[test]
    fn loads_from_url() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                    super::CONFIG_WORKSPACE_FILENAME,
r#"
extends: https://raw.githubusercontent.com/moonrepo/moon/master/tests/fixtures/config-extends/.moon/workspace.yml

node:
    version: '18.0.0'
    npm:
        version: '8.0.0'
"#,
                )?;

            let config: WorkspaceConfig = super::load_jailed_config(jail.directory())?;

            // Inherits from extended file
            assert!(!config.node.as_ref().unwrap().add_engines_constraint);
            assert!(!config.typescript.unwrap().sync_project_references);
            assert_eq!(config.vcs.manager, VcsManager::Svn);

            // Ensure we can override the extended config
            assert_eq!(config.node.as_ref().unwrap().version, "18.0.0".to_owned());
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
    //             super::CONFIG_WORKSPACE_FILENAME,
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
                CONFIG_WORKSPACE_FILENAME,
                r#"
projects: {}
node:
    packageManager: yarn"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config,
                WorkspaceConfig {
                    node: Some(NodeConfig {
                        package_manager: NodePackageManager::Yarn,
                        ..NodeConfig::default()
                    }),
                    projects: WorkspaceProjects::default(),
                    ..WorkspaceConfig::default()
                }
            );

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected struct NodeConfig for key \"workspace.node\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_WORKSPACE_FILENAME, "node: 123")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid semantic version for key \"workspace.node.version\""
    )]
    fn invalid_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
  version: 'foo bar'
projects:
  foo: packages/foo"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid semantic version for key \"workspace.node.version\""
    )]
    fn no_patch_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
  version: '16.13'
projects:
  foo: packages/foo"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid semantic version for key \"workspace.node.version\""
    )]
    fn no_minor_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
  version: '16'
projects:
  foo: packages/foo"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "unknown variant: found `what`, expected `one of `npm`, `pnpm`, `yarn`` for key \"workspace.node.packageManager\""
    )]
    fn invalid_package_manager() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
  version: '16.13.0'
  packageManager: what
projects:
  foo: packages/foo"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    fn valid_package_manager() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
  version: '16.13.0'
  packageManager: yarn
projects:
  foo: packages/foo"#,
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
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
    version: '16.13.0'
projects: {}
"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(config.node.unwrap().version, String::from("4.5.6"));

            Ok(())
        });
    }
}

mod npm {
    #[test]
    #[should_panic(
        expected = "invalid type: found string \"foo\", expected struct NpmConfig for key \"workspace.node.npm\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
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
        expected = "Must be a valid semantic version for key \"workspace.node.npm.version\""
    )]
    fn invalid_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
    version: '16.13.0'
    npm:
        version: 'foo bar'
projects:
  foo: packages/foo
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
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
    version: '16.13.0'
    npm:
        version: '1.2.3'
projects: {}
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
        expected = "invalid type: found string \"foo\", expected struct PnpmConfig for key \"workspace.node.pnpm\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
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
        expected = "Must be a valid semantic version for key \"workspace.node.pnpm.version\""
    )]
    fn invalid_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
    version: '16.13.0'
    pnpm:
        version: 'foo bar'
projects:
  foo: packages/foo"#,
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
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
    version: '16.13.0'
    packageManager: 'pnpm'
    pnpm:
        version: '1.2.3'
projects: {}
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
        expected = "invalid type: found string \"foo\", expected struct YarnConfig for key \"workspace.node.yarn\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
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
        expected = "Must be a valid semantic version for key \"workspace.node.yarn.version\""
    )]
    fn invalid_version() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
    version: '16.13.0'
    yarn:
        version: 'foo bar'
projects:
  foo: packages/foo"#,
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
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
node:
    version: '16.13.0'
    packageManager: 'yarn'
    yarn:
        version: '1.2.3'
projects: {}
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

mod projects {
    use super::*;
    use std::collections::HashMap;

    #[test]
    #[should_panic(
        expected = "expected a sequence of globs or a map of projects for key \"workspace.projects\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_WORKSPACE_FILENAME, "projects: apps/*")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "Absolute paths are not supported for key \"workspace.projects\"")]
    fn no_abs_paths() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects:
  app: /apps/app
  foo: packages/foo"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Parent relative paths are not supported for key \"workspace.projects\""
    )]
    fn no_parent_paths() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects:
  app: ../apps/app
  foo: packages/foo"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    fn valid_list() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects:
  app: apps/app
  foo: ./packages/foo"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config.projects,
                WorkspaceProjects::Sources(HashMap::from([
                    (String::from("app"), String::from("apps/app")),
                    (String::from("foo"), String::from("./packages/foo"))
                ])),
            );

            Ok(())
        });
    }

    #[test]
    fn supports_globs() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects:
    - 'apps/*'
    - 'packages/*'"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config.projects,
                WorkspaceProjects::Globs(moon_utils::string_vec!["apps/*", "packages/*"])
            );

            Ok(())
        });
    }
}

mod vcs {
    use super::*;

    #[test]
    fn loads_defaults() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                CONFIG_WORKSPACE_FILENAME,
                r#"
projects: {}
vcs:
    manager: svn"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config,
                WorkspaceConfig {
                    projects: WorkspaceProjects::default(),
                    vcs: VcsConfig {
                        manager: VcsManager::Svn,
                        ..VcsConfig::default()
                    },
                    ..WorkspaceConfig::default()
                }
            );

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected struct VcsConfig for key \"workspace.vcs\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects: {}
vcs: 123"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "unknown variant: found `unknown`, expected ``git` or `svn`` for key \"workspace.vcs.manager\""
    )]
    fn invalid_manager_option() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects: {}
vcs:
    manager: unknown"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a string for key \"workspace.vcs.defaultBranch\""
    )]
    fn invalid_default_branch_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects: {}
vcs:
    defaultBranch: 123"#,
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }
}

mod generator {

    #[test]
    #[should_panic(expected = "At least 1 template path is required")]
    fn empty_templates() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                "generator:\n  templates: []",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "Parent relative paths are not supported")]
    fn no_parent_relative() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                "generator:\n  templates: ['../templates']",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }
}
