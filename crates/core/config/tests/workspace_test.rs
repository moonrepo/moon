use moon_config::{
    ConfigError, GeneratorConfig, HasherConfig, NotifierConfig, RunnerConfig, VcsConfig,
    VcsManager, WorkspaceConfig, WorkspaceProjects,
};
use moon_constants::CONFIG_WORKSPACE_FILENAME;
use moon_test_utils::get_fixtures_path;
use std::path::Path;

fn load_jailed_config(root: &Path) -> Result<WorkspaceConfig, figment::Error> {
    match WorkspaceConfig::load(root.join(CONFIG_WORKSPACE_FILENAME)) {
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
        jail.create_file(CONFIG_WORKSPACE_FILENAME, "projects: {}")?;

        let config = load_jailed_config(jail.directory())?;

        assert_eq!(
            config,
            WorkspaceConfig {
                runner: RunnerConfig::default(),
                generator: GeneratorConfig::default(),
                extends: None,
                hasher: HasherConfig::default(),
                notifier: NotifierConfig::default(),
                projects: WorkspaceProjects::default(),
                vcs: VcsConfig::default(),
                version_constraint: None,
                schema: String::new(),
            }
        );

        Ok(())
    });
}

mod extends {
    use super::*;
    use moon_test_utils::pretty_assertions::assert_eq;
    use std::fs;

    #[test]
    fn recursive_merges() {
        let fixture = get_fixtures_path("config-extends/workspace");
        let config = WorkspaceConfig::load(fixture.join("base-2.yml")).unwrap();

        assert_eq!(
            config,
            WorkspaceConfig {
                runner: RunnerConfig {
                    cache_lifetime: "3 hours".into(),
                    log_running_command: false,
                    ..RunnerConfig::default()
                },
                vcs: VcsConfig {
                    manager: VcsManager::Svn,
                    ..VcsConfig::default()
                },
                ..WorkspaceConfig::default()
            }
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
}

mod projects {
    use super::*;
    use rustc_hash::FxHashMap;

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
    #[should_panic(expected = "Absolute paths are not supported for key \"workspace.projects\"")]
    fn no_abs_paths_when_nested() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects:
  globs: []
  sources:
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
    #[should_panic(
        expected = "Parent relative paths are not supported for key \"workspace.projects\""
    )]
    fn no_parent_paths_when_nested() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects:
  globs: []
  sources:
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
  foo-kebab: ./packages/foo
  barCamel: ./packages/bar
  baz_snake: ./packages/baz
  qux.dot: ./packages/qux
  wat/slash: ./packages/wat
"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config.projects,
                WorkspaceProjects::Sources(FxHashMap::from_iter([
                    (String::from("app"), String::from("apps/app")),
                    (String::from("foo-kebab"), String::from("./packages/foo")),
                    (String::from("barCamel"), String::from("./packages/bar")),
                    (String::from("baz_snake"), String::from("./packages/baz")),
                    (String::from("qux.dot"), String::from("./packages/qux")),
                    (String::from("wat/slash"), String::from("./packages/wat"))
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

    #[test]
    fn supports_globs_when_nested() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects:
  sources: {}
  globs:
    - 'apps/*'
    - 'packages/*'"#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config.projects,
                WorkspaceProjects::Both {
                    globs: moon_utils::string_vec!["apps/*", "packages/*"],
                    sources: FxHashMap::default()
                }
            );

            Ok(())
        });
    }

    #[test]
    fn supports_nested_both_syntax() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                r#"
projects:
  globs:
    - 'apps/*'
    - 'packages/*'
  sources:
    app: apps/app "#,
            )?;

            let config = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config.projects,
                WorkspaceProjects::Both {
                    globs: moon_utils::string_vec!["apps/*", "packages/*"],
                    sources: FxHashMap::from_iter([(
                        String::from("app"),
                        String::from("apps/app")
                    ),])
                }
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

mod version_constraint {
    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a string for key \"workspace.versionConstraint\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_WORKSPACE_FILENAME, "versionConstraint: 123")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid semantic version requirement or range for key \"workspace.versionConstraint\""
    )]
    fn invalid_req() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_WORKSPACE_FILENAME,
                "versionConstraint: '@1.0.0'",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }
}
