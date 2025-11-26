mod utils;

use httpmock::prelude::*;
use moon_common::Id;
use moon_config::{
    ConfigLoader, FilePath, GlobPath, TemplateLocator, VcsProvider, WorkspaceConfig,
    WorkspaceProjectGlobFormat, WorkspaceProjects,
};
use rustc_hash::FxHashMap;
use schematic::ConfigLoader as BaseLoader;
use semver::Version;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::path::Path;
use utils::*;

const FILENAME: &str = ".moon/workspace.yml";

fn load_config_from_file(path: &Path) -> WorkspaceConfig {
    BaseLoader::<WorkspaceConfig>::new()
        .file(path)
        .unwrap()
        .load()
        .unwrap()
        .config
}

fn load_config_from_root(root: &Path) -> miette::Result<WorkspaceConfig> {
    ConfigLoader::new(root.join(".moon")).load_workspace_config(root)
}

mod workspace_config {
    use super::*;

    #[test]
    #[should_panic(expected = "unknown field `unknown`, expected one of `$schema`")]
    fn error_unknown_field() {
        test_load_config(FILENAME, "unknown: 123", load_config_from_root);
    }

    #[test]
    fn loads_defaults() {
        let config = test_load_config(FILENAME, "{}", load_config_from_root);

        assert!(config.telemetry);
        assert!(config.version_constraint.is_none());
    }

    mod extends {
        use super::*;

        const SHARED_WORKSPACE: &str = r"
projects:
    - packages/*
";

        #[test]
        fn recursive_merges() {
            let sandbox = create_sandbox("extends/workspace");
            let config = test_config(sandbox.path().join("base-2.yml"), |path| {
                Ok(load_config_from_file(path))
            });

            assert_eq!(config.pipeline.cache_lifetime, "3 hours");
            assert!(!config.pipeline.log_running_command);
            assert_eq!(config.vcs.provider, VcsProvider::Bitbucket);
        }

        #[test]
        #[should_panic(expected = "only file paths and URLs can be extended")]
        fn not_a_url_or_file() {
            test_load_config(FILENAME, "extends: 'random value'", |path| {
                load_config_from_root(path)
            });
        }

        #[test]
        #[should_panic(expected = "only secure URLs can be extended")]
        fn not_a_https_url() {
            test_load_config(
                FILENAME,
                "extends: 'http://domain.com/config.yml'",
                load_config_from_root,
            );
        }

        #[test]
        #[should_panic(expected = "no matching source format")]
        fn not_a_yaml_file() {
            test_load_config(FILENAME, "extends: './file.txt'", |path| {
                std::fs::write(path.join(".moon/file.txt"), "").unwrap();
                load_config_from_root(path)
            });
        }

        #[test]
        #[should_panic(expected = "no matching source format")]
        fn not_a_yaml_url() {
            test_load_config(
                FILENAME,
                "extends: 'https://domain.com/config.txt'",
                load_config_from_root,
            );
        }

        #[test]
        fn loads_from_url() {
            let sandbox = create_empty_sandbox();
            let server = MockServer::start();

            server.mock(|when, then| {
                when.method(GET).path("/config.yml");
                then.status(200).body(SHARED_WORKSPACE);
            });

            let url = server.url("/config.yml");

            sandbox.create_file(
                "workspace.yml",
                format!(
                    r"
extends: '{url}'

telemetry: false
"
                ),
            );

            let config = test_config(sandbox.path().join("workspace.yml"), |path| {
                Ok(load_config_from_file(path))
            });

            if let WorkspaceProjects::Globs(globs) = config.projects {
                assert_eq!(globs, vec!["packages/*".to_owned()]);
            } else {
                panic!();
            }

            assert!(!config.telemetry);
        }
    }

    mod default_project {
        use super::*;

        #[test]
        fn can_set() {
            let config = test_load_config(
                FILENAME,
                r"
defaultProject: app
",
                load_config_from_root,
            );

            assert_eq!(config.default_project.unwrap(), "app");
        }

        #[test]
        #[should_panic(expected = "Invalid identifier format for")]
        fn errors_if_empty() {
            test_load_config(
                FILENAME,
                r"
defaultProject: ''
",
                load_config_from_root,
            );
        }

        #[test]
        #[should_panic(expected = "Invalid identifier format for")]
        fn errors_if_invalid_format() {
            test_load_config(
                FILENAME,
                r"
defaultProject: 'nsN@d0n02OS'
",
                load_config_from_root,
            );
        }
    }

    mod projects {
        use super::*;

        #[test]
        fn supports_sources() {
            let config = test_load_config(
                FILENAME,
                r"
projects:
  app: apps/app
  foo-kebab: ./packages/foo
  barCamel: packages/bar
  baz_snake: ./packages/baz
  qux.dot: packages/qux
  wat/slash: ./packages/wat
",
                load_config_from_root,
            );

            match config.projects {
                WorkspaceProjects::Sources(map) => {
                    assert_eq!(
                        map,
                        FxHashMap::from_iter([
                            (Id::raw("app"), "apps/app".into()),
                            (Id::raw("foo-kebab"), "./packages/foo".into()),
                            (Id::raw("barCamel"), "packages/bar".into()),
                            (Id::raw("baz_snake"), "./packages/baz".into()),
                            (Id::raw("qux.dot"), "packages/qux".into()),
                            (Id::raw("wat/slash"), "./packages/wat".into())
                        ]),
                    );
                }
                _ => panic!(),
            };
        }

        #[test]
        #[should_panic(expected = "absolute paths are not supported")]
        fn errors_on_absolute_sources() {
            test_load_config(
                FILENAME,
                r"
projects:
  app: /apps/app
",
                load_config_from_root,
            );
        }

        #[test]
        #[should_panic(expected = "parent directory traversal (..) is not supported")]
        fn errors_on_parent_sources() {
            test_load_config(
                FILENAME,
                r"
projects:
  app: ../apps/app
",
                load_config_from_root,
            );
        }

        #[test]
        #[should_panic(expected = "globs are not supported, expected a literal file path")]
        fn errors_on_glob_in_sources() {
            test_load_config(
                FILENAME,
                r"
projects:
  app: apps/app/*
",
                load_config_from_root,
            );
        }

        #[test]
        fn supports_globs() {
            let config = test_load_config(
                FILENAME,
                r"
projects:
  - apps/*
  - packages/*
  - internal
",
                load_config_from_root,
            );

            match config.projects {
                WorkspaceProjects::Globs(list) => {
                    assert_eq!(
                        list,
                        vec![
                            "apps/*".to_owned(),
                            "packages/*".to_owned(),
                            "internal".to_owned(),
                        ],
                    );
                }
                _ => panic!(),
            };
        }

        #[test]
        #[should_panic(expected = "absolute paths are not supported")]
        fn errors_on_absolute_globs() {
            test_load_config(
                FILENAME,
                r"
projects:
  - /apps/*
",
                load_config_from_root,
            );
        }

        #[test]
        #[should_panic(expected = "parent directory traversal (..) is not supported")]
        fn errors_on_parent_globs() {
            test_load_config(
                FILENAME,
                r"
projects:
  - ../apps/*
",
                load_config_from_root,
            );
        }

        #[test]
        fn supports_globs_and_projects() {
            let config = test_load_config(
                FILENAME,
                r"
projects:
  sources:
    app: app
  globs:
    - packages/*
",
                load_config_from_root,
            );

            match config.projects {
                WorkspaceProjects::Both(cfg) => {
                    assert_eq!(cfg.globs, vec!["packages/*".to_owned()]);
                    assert_eq!(
                        cfg.sources,
                        FxHashMap::from_iter([(Id::raw("app"), "app".into())])
                    );
                }
                _ => panic!(),
            };
        }

        #[test]
        fn supports_globs_with_format() {
            let config = test_load_config(
                FILENAME,
                r"
projects:
  globFormat: source-path
  globs:
    - packages/*
",
                load_config_from_root,
            );

            match config.projects {
                WorkspaceProjects::Both(cfg) => {
                    assert_eq!(cfg.glob_format, WorkspaceProjectGlobFormat::SourcePath);
                    assert_eq!(cfg.globs, vec!["packages/*".to_owned()]);
                    assert_eq!(cfg.sources, FxHashMap::default());
                }
                _ => panic!(),
            };
        }
    }

    mod constraints {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "constraints: {}", |path| {
                load_config_from_root(path)
            });

            assert!(config.constraints.enforce_layer_relationships);
            assert!(config.constraints.tag_relationships.is_empty());
        }

        #[test]
        fn can_set_tags() {
            let config = test_load_config(
                FILENAME,
                r"
constraints:
  tagRelationships:
    id: ['other']
",
                load_config_from_root,
            );

            assert!(config.constraints.enforce_layer_relationships);
            assert_eq!(
                config.constraints.tag_relationships,
                FxHashMap::from_iter([(Id::raw("id"), vec![Id::raw("other")])])
            );
        }

        #[test]
        #[should_panic(
            expected = "invalid type: integer `123`, expected struct PartialConstraintsConfig"
        )]
        fn errors_on_invalid_type() {
            test_load_config(FILENAME, "constraints: 123", |path| {
                load_config_from_root(path)
            });
        }

        #[test]
        #[should_panic(expected = "invalid type: string \"abc\", expected a boolean")]
        fn errors_on_invalid_setting_type() {
            test_load_config(
                FILENAME,
                r"
constraints:
  enforceLayerRelationships: abc
",
                load_config_from_root,
            );
        }

        #[test]
        #[should_panic(expected = "Invalid identifier format for `bad id`")]
        fn errors_on_invalid_tag_format() {
            test_load_config(
                FILENAME,
                r"
constraints:
  tagRelationships:
    id: ['bad id']
",
                load_config_from_root,
            );
        }
    }

    mod generator {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "generator: {}", |path| {
                load_config_from_root(path)
            });

            assert_eq!(
                config.generator.templates,
                vec![TemplateLocator::File {
                    path: FilePath("./templates".into())
                }]
            );
        }

        #[test]
        fn can_set_templates() {
            let config = test_load_config(
                FILENAME,
                r"
generator:
  templates:
    - custom/path
    - ./rel/path
    - /abs/path
",
                load_config_from_root,
            );

            assert_eq!(
                config.generator.templates,
                vec![
                    TemplateLocator::File {
                        path: FilePath("custom/path".into())
                    },
                    TemplateLocator::File {
                        path: FilePath("rel/path".into())
                    },
                    // TemplateLocator::File {
                    //     path: FilePath("../parent/path".into())
                    // },
                    TemplateLocator::File {
                        path: FilePath("/abs/path".into())
                    }
                ]
            );
        }

        #[test]
        fn can_set_url_locations() {
            let config = test_load_config(
                FILENAME,
                r"
generator:
  templates:
    - https://download.com/some/file.zip
",
                load_config_from_root,
            );

            assert_eq!(
                config.generator.templates,
                vec![TemplateLocator::Archive {
                    url: "https://download.com/some/file.zip".into()
                },]
            );
        }

        #[test]
        fn can_set_git_locations() {
            let config = test_load_config(
                FILENAME,
                r"
generator:
  templates:
    - git://github.com/org/repo#master
    - git://gitlab.com/org/repo#main
    - git://ghe.self.hosted.com/some/org/repo#v1.2.3
",
                load_config_from_root,
            );

            assert_eq!(
                config.generator.templates,
                vec![
                    TemplateLocator::Git {
                        remote_url: "github.com/org/repo".into(),
                        revision: "master".into()
                    },
                    TemplateLocator::Git {
                        remote_url: "gitlab.com/org/repo".into(),
                        revision: "main".into()
                    },
                    TemplateLocator::Git {
                        remote_url: "ghe.self.hosted.com/some/org/repo".into(),
                        revision: "v1.2.3".into()
                    },
                ]
            );
        }

        #[test]
        fn can_set_npm_locations() {
            let config = test_load_config(
                FILENAME,
                r"
generator:
  templates:
    - npm://package-name#1.2.3
    - npm://@scope/package#4.5.6
",
                load_config_from_root,
            );

            assert_eq!(
                config.generator.templates,
                vec![
                    TemplateLocator::Npm {
                        package: "package-name".into(),
                        version: Version::new(1, 2, 3)
                    },
                    TemplateLocator::Npm {
                        package: "@scope/package".into(),
                        version: Version::new(4, 5, 6)
                    }
                ]
            );
        }

        #[test]
        fn can_set_glob_locations() {
            let config = test_load_config(
                FILENAME,
                r"
generator:
  templates:
    - ./templates/*
    - glob://common/*/templates/*
",
                load_config_from_root,
            );

            assert_eq!(
                config.generator.templates,
                vec![
                    TemplateLocator::Glob {
                        glob: GlobPath("templates/*".into())
                    },
                    TemplateLocator::Glob {
                        glob: GlobPath("common/*/templates/*".into())
                    },
                ]
            );
        }

        #[test]
        #[should_panic(
            expected = "Invalid URL template locator, must contain a trailing file name with a supported archive extension"
        )]
        fn errors_for_invalid_url_ext() {
            test_load_config(
                FILENAME,
                r"
generator:
  templates: ['https://download.com/some/file.png']
",
                load_config_from_root,
            );
        }

        #[test]
        #[should_panic(
            expected = "Invalid Git template locator, must be in the format of `git://url#revision`"
        )]
        fn errors_for_no_git_revision() {
            test_load_config(
                FILENAME,
                r"
generator:
  templates: ['git://github.com/org/repo']
",
                load_config_from_root,
            );
        }

        #[test]
        #[should_panic(
            expected = "Invalid npm template locator, must be in the format of `npm://package#version`"
        )]
        fn errors_for_no_npm_version() {
            test_load_config(
                FILENAME,
                r"
generator:
  templates: ['npm://@scope/package']
",
                load_config_from_root,
            );
        }
    }

    mod hasher {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "hasher: {}", load_config_from_root);

            assert!(config.hasher.warn_on_missing_inputs);
        }

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                FILENAME,
                r"
hasher:
  warnOnMissingInputs: false
",
                load_config_from_root,
            );

            assert!(!config.hasher.warn_on_missing_inputs);
        }

        #[test]
        #[should_panic(expected = "unknown variant `unknown`, expected `glob` or `vcs`")]
        fn errors_on_invalid_variant() {
            test_load_config(
                FILENAME,
                r"
hasher:
  walkStrategy: unknown
",
                load_config_from_root,
            );
        }
    }

    mod notifier {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "notifier: {}", load_config_from_root);

            assert!(config.notifier.webhook_url.is_none());
        }

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                FILENAME,
                r"
notifier:
  webhookUrl: 'https://domain.com/some/url'
",
                load_config_from_root,
            );

            assert_eq!(
                config.notifier.webhook_url,
                Some("https://domain.com/some/url".into())
            );
        }

        #[test]
        #[should_panic(expected = "not a valid url: relative URL without a base")]
        fn errors_on_invalid_url() {
            test_load_config(
                FILENAME,
                r"
notifier:
  webhookUrl: 'invalid value'
",
                load_config_from_root,
            );
        }

        #[test]
        #[should_panic(expected = "only secure URLs are allowed")]
        fn errors_on_non_https_url() {
            test_load_config(
                FILENAME,
                r"
notifier:
  webhookUrl: 'http://domain.com/some/url'
",
                load_config_from_root,
            );
        }
    }

    mod runner {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "pipeline: {}", load_config_from_root);

            assert_eq!(config.pipeline.cache_lifetime, "7 days");
            assert!(config.pipeline.inherit_colors_for_piped_tasks);
        }

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                FILENAME,
                r"
pipeline:
  cacheLifetime: 10 hours
  inheritColorsForPipedTasks: false
",
                load_config_from_root,
            );

            assert_eq!(config.pipeline.cache_lifetime, "10 hours");
            assert!(!config.pipeline.inherit_colors_for_piped_tasks);
        }
    }

    mod vcs {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "vcs: {}", load_config_from_root);

            assert_eq!(config.vcs.default_branch, "master");
            assert_eq!(
                config.vcs.remote_candidates,
                vec!["origin".to_string(), "upstream".to_string()]
            );
        }

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                FILENAME,
                r"
vcs:
  defaultBranch: main
  remoteCandidates: [next]
",
                load_config_from_root,
            );

            assert_eq!(config.vcs.default_branch, "main");
            assert_eq!(config.vcs.remote_candidates, vec!["next".to_string()]);
        }

        #[test]
        #[should_panic(expected = "unknown variant `mercurial`, expected `git`")]
        fn errors_on_invalid_client() {
            test_load_config(
                FILENAME,
                r"
vcs:
  client: mercurial
",
                load_config_from_root,
            );
        }
    }

    mod version_constraint {
        use super::*;

        #[test]
        #[should_panic(expected = "unexpected character '@' while parsing major version number")]
        fn errors_on_invalid_req() {
            test_load_config(FILENAME, "versionConstraint: '@1.0.0'", |path| {
                load_config_from_root(path)
            });
        }
    }

    #[test]
    fn supports_hcl() {
        load_workspace_config_in_format("hcl");
    }

    #[test]
    fn supports_pkl() {
        load_workspace_config_in_format("pkl");
    }
}
