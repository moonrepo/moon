use moon_codegen::CodeGenerator;
use moon_common::Id;
use moon_config::{
    FilePath, GeneratorConfig, GlobPath, TemplateLocator, TemplateVariable, Version,
};
use moon_env::MoonEnvironment;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::sync::Arc;

mod codegen {
    use super::*;

    mod create_template {
        use super::*;

        #[test]
        #[should_panic(expected = "A template with the name standard already exists")]
        fn errors_if_already_exists() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("templates/standard/file", "");

            CodeGenerator::new(
                sandbox.path(),
                &GeneratorConfig::default(),
                MoonEnvironment::new_testing(sandbox.path()).into(),
            )
            .create_template("standard")
            .unwrap();
        }

        #[test]
        fn creates_the_template() {
            let sandbox = create_empty_sandbox();

            let template = CodeGenerator::new(
                sandbox.path(),
                &GeneratorConfig::default(),
                MoonEnvironment::new_testing(sandbox.path()).into(),
            )
            .create_template("new-template")
            .unwrap();

            assert!(sandbox.path().join("templates/new-template").exists());
            assert!(
                sandbox
                    .path()
                    .join("templates/new-template/template.yml")
                    .exists()
            );

            assert_eq!(template.id, "new-template");
            assert_eq!(template.root, sandbox.path().join("templates/new-template"));
        }

        #[test]
        fn creates_the_template_from_another_dir() {
            let sandbox = create_empty_sandbox();

            let template = CodeGenerator::new(
                sandbox.path(),
                &GeneratorConfig {
                    templates: vec![TemplateLocator::File {
                        path: FilePath("./scaffolding".to_owned()),
                    }],
                },
                MoonEnvironment::new_testing(sandbox.path()).into(),
            )
            .create_template("new-template")
            .unwrap();

            assert!(sandbox.path().join("scaffolding/new-template").exists());
            assert!(
                sandbox
                    .path()
                    .join("scaffolding/new-template/template.yml")
                    .exists()
            );

            assert_eq!(template.id, "new-template");
            assert_eq!(
                template.root,
                sandbox.path().join("scaffolding/new-template")
            );
        }

        #[test]
        fn cleans_and_formats_the_name() {
            let sandbox = create_empty_sandbox();

            let template = CodeGenerator::new(
                sandbox.path(),
                &GeneratorConfig::default(),
                MoonEnvironment::new_testing(sandbox.path()).into(),
            )
            .create_template("so&me temPlatE- with Ran!dom-Valu^es 123_")
            .unwrap();

            assert!(
                sandbox
                    .path()
                    .join("templates/so-me-temPlatE--with-Ran-dom-Valu-es-123_")
                    .exists()
            );
            assert!(
                sandbox
                    .path()
                    .join("templates/so-me-temPlatE--with-Ran-dom-Valu-es-123_/template.yml")
                    .exists()
            );

            assert_eq!(template.id, "so-me-temPlatE--with-Ran-dom-Valu-es-123_");
            assert_eq!(
                template.root,
                sandbox
                    .path()
                    .join("templates/so-me-temPlatE--with-Ran-dom-Valu-es-123_")
            );
        }
    }

    mod load_templates {
        use super::*;

        #[tokio::test]
        async fn clones_a_git_repo() {
            let sandbox = create_empty_sandbox();
            let env = Arc::new(MoonEnvironment::new_testing(sandbox.path()));
            let config = GeneratorConfig {
                templates: vec![TemplateLocator::Git {
                    remote_url: "github.com/moonrepo/moon-configs.git".into(),
                    revision: "master".into(),
                }],
            };

            let mut codegen = CodeGenerator::new(sandbox.path(), &config, Arc::clone(&env));
            codegen.load_templates().await.unwrap();

            assert!(codegen.template_locations[0].starts_with(&env.templates_dir));
            assert!(
                env.templates_dir
                    .join("github.com")
                    .join("moonrepo")
                    .join("moon-configs.git")
                    .exists()
            );
        }

        #[tokio::test]
        async fn downloads_an_npm_package() {
            let sandbox = create_empty_sandbox();
            let env = Arc::new(MoonEnvironment::new_testing(sandbox.path()));
            let config = GeneratorConfig {
                templates: vec![TemplateLocator::Npm {
                    package: "@moonrepo/cli".into(),
                    version: Version::new(1, 0, 0),
                }],
            };

            let mut codegen = CodeGenerator::new(sandbox.path(), &config, Arc::clone(&env));
            codegen.load_templates().await.unwrap();

            assert!(codegen.template_locations[0].starts_with(&env.templates_dir));
            assert!(
                env.templates_dir
                    .join("npm")
                    .join("moonrepo_cli")
                    .join("1.0.0")
                    .exists()
            );
        }

        #[tokio::test]
        async fn walks_with_globs() {
            let sandbox = create_sandbox("include");
            let env = Arc::new(MoonEnvironment::new_testing(sandbox.path()));
            let config = GeneratorConfig {
                templates: vec![TemplateLocator::Glob {
                    glob: GlobPath::try_from("./templates/*").unwrap(),
                }],
            };

            let mut codegen = CodeGenerator::new(sandbox.path(), &config, Arc::clone(&env));
            codegen.load_templates().await.unwrap();

            codegen.template_locations.sort();

            assert_eq!(
                codegen.template_locations,
                vec![
                    sandbox.path().join("templates/base"),
                    sandbox.path().join("templates/extended"),
                    sandbox.path().join("templates/partials"),
                ]
            );
        }

        #[tokio::test]
        #[should_panic(expected = "Found multiple templates with the same name folder-name")]
        async fn errors_for_dupe_ids() {
            let sandbox = create_sandbox("dupes");
            let config = GeneratorConfig::default();

            let mut codegen = CodeGenerator::new(
                sandbox.path(),
                &config,
                MoonEnvironment::new_testing(sandbox.path()).into(),
            );
            codegen.load_templates().await.unwrap();
            codegen.get_template("three").unwrap();
        }
    }

    mod get_template {
        use super::*;

        #[tokio::test]
        async fn loads_by_name() {
            let sandbox = create_sandbox("generator");
            let config = GeneratorConfig::default();

            let mut codegen = CodeGenerator::new(
                sandbox.path(),
                &config,
                MoonEnvironment::new_testing(sandbox.path()).into(),
            );
            codegen.load_templates().await.unwrap();

            let template = codegen.get_template("one").unwrap();

            assert_eq!(template.id, Id::raw("one"));
            assert_eq!(template.root, sandbox.path().join("templates/one"));
        }

        #[tokio::test]
        #[should_panic(expected = "No template with the name three could be found")]
        async fn errors_for_missing() {
            let sandbox = create_sandbox("generator");
            let config = GeneratorConfig::default();

            let mut codegen = CodeGenerator::new(
                sandbox.path(),
                &config,
                MoonEnvironment::new_testing(sandbox.path()).into(),
            );
            codegen.load_templates().await.unwrap();
            codegen.get_template("three").unwrap();
        }

        mod extends {
            use super::*;

            #[tokio::test]
            async fn loads_extended() {
                let sandbox = create_sandbox("generator");
                let config = GeneratorConfig::default();

                let mut codegen = CodeGenerator::new(
                    sandbox.path(),
                    &config,
                    MoonEnvironment::new_testing(sandbox.path()).into(),
                );
                codegen.load_templates().await.unwrap();
                let template = codegen.get_template("extends").unwrap();

                assert_eq!(template.id, Id::raw("extends"));
                assert_eq!(template.root, sandbox.path().join("templates/extends"));

                assert_eq!(template.templates[0].id, Id::raw("extends-from-a"));
                assert_eq!(
                    template.templates[0].root,
                    sandbox.path().join("templates/extends-from-a")
                );

                assert_eq!(template.templates[1].id, Id::raw("extends-from-b"));
                assert_eq!(
                    template.templates[1].root,
                    sandbox.path().join("templates/extends-from-b")
                );

                assert_eq!(
                    template.templates[1].templates[0].id,
                    Id::raw("extends-from-c")
                );
                assert_eq!(
                    template.templates[1].templates[0].root,
                    sandbox.path().join("templates/extends-from-c")
                );
            }

            #[tokio::test]
            async fn inherits_extended_variables() {
                let sandbox = create_sandbox("generator");
                let config = GeneratorConfig::default();

                let mut codegen = CodeGenerator::new(
                    sandbox.path(),
                    &config,
                    MoonEnvironment::new_testing(sandbox.path()).into(),
                );
                codegen.load_templates().await.unwrap();
                let template = codegen.get_template("extends").unwrap();

                assert_eq!(
                    template.config.variables.keys().collect::<Vec<_>>(),
                    vec!["a", "base", "c", "b"]
                );

                // Test that the base vars aren't overwritten
                let result = matches!(
                    template.config.variables.get("c").unwrap(),
                    // c template is a string
                    TemplateVariable::Boolean(_)
                );

                assert!(result);
            }

            #[tokio::test]
            #[should_panic(expected = "No template with the name missing could be found")]
            async fn errors_for_missing_extends() {
                let sandbox = create_sandbox("generator");
                let config = GeneratorConfig::default();

                let mut codegen = CodeGenerator::new(
                    sandbox.path(),
                    &config,
                    MoonEnvironment::new_testing(sandbox.path()).into(),
                );
                codegen.load_templates().await.unwrap();
                codegen.get_template("extends-unknown").unwrap();
            }
        }
    }
}
