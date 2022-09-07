use moon_config::{GeneratorConfig, TemplateConfig};
use moon_generator::{Generator, Template};
use moon_utils::{string_vec, test::create_sandbox};

mod create_template {
    use super::*;

    #[tokio::test]
    #[should_panic(expected = "ExistingTemplate(\"standard\"")]
    async fn errors_if_already_exists() {
        let dir = create_sandbox("generator");

        Generator::create(dir.path(), &GeneratorConfig::default())
            .unwrap()
            .create_template("standard")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn creates_the_template() {
        let dir = create_sandbox("generator");

        let template = Generator::create(dir.path(), &GeneratorConfig::default())
            .unwrap()
            .create_template("new-template")
            .await
            .unwrap();

        assert!(dir.join("templates/new-template").exists());
        assert!(dir.join("templates/new-template/template.yml").exists());

        assert_eq!(
            template,
            Template {
                config: TemplateConfig {
                    title: "Title".into(),
                    description: "Description of the template.".into(),
                },
                files: vec![],
                name: "new-template".into(),
                root: dir.join("templates/new-template")
            }
        );
    }

    #[tokio::test]
    async fn creates_the_template_from_another_dir() {
        let dir = create_sandbox("generator");

        let template = Generator::create(
            dir.path(),
            &GeneratorConfig {
                templates: string_vec!["./scaffolding"],
            },
        )
        .unwrap()
        .create_template("new-template")
        .await
        .unwrap();

        assert!(dir.join("scaffolding/new-template").exists());
        assert!(dir.join("scaffolding/new-template/template.yml").exists());

        assert_eq!(
            template,
            Template {
                config: TemplateConfig {
                    title: "Title".into(),
                    description: "Description of the template.".into(),
                },
                files: vec![],
                name: "new-template".into(),
                root: dir.join("scaffolding/new-template")
            }
        );
    }

    #[tokio::test]
    async fn cleans_and_formats_the_name() {
        let dir = create_sandbox("generator");

        let template = Generator::create(dir.path(), &GeneratorConfig::default())
            .unwrap()
            .create_template("so&me temPlatE- with Ran!dom-Valu^es 123_")
            .await
            .unwrap();

        assert!(dir
            .join("templates/sometemPlatE-withRandom-Values123_")
            .exists());
        assert!(dir
            .join("templates/sometemPlatE-withRandom-Values123_/template.yml")
            .exists());

        assert_eq!(
            template,
            Template {
                config: TemplateConfig {
                    title: "Title".into(),
                    description: "Description of the template.".into(),
                },
                files: vec![],
                name: "sometemPlatE-withRandom-Values123_".into(),
                root: dir.join("templates/sometemPlatE-withRandom-Values123_")
            }
        );
    }
}
