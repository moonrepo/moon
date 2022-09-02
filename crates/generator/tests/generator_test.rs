use moon_utils::fs;

mod generate_template {
    use super::*;
    use moon_config::GeneratorConfig;
    use moon_generator::{Generator, Template};
    use moon_utils::{
        string_vec,
        test::{get_fixtures_dir, get_fixtures_root},
    };

    async fn create_templates_dirs(name: &str) -> assert_fs::TempDir {
        let dir = assert_fs::TempDir::new().unwrap();

        fs::copy_dir_all(
            get_fixtures_root(),
            get_fixtures_dir("template"),
            dir.path().join(name),
        )
        .await
        .unwrap();

        dir
    }

    #[tokio::test]
    #[should_panic(expected = "ExistingTemplate(\"template\"")]
    async fn errors_if_already_exists() {
        let dir = create_templates_dirs("templates").await;

        Generator::create(dir.path(), &GeneratorConfig::default())
            .unwrap()
            .generate_template("template")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn creates_the_template() {
        let dir = create_templates_dirs("templates").await;

        let template = Generator::create(dir.path(), &GeneratorConfig::default())
            .unwrap()
            .generate_template("new-template")
            .await
            .unwrap();

        assert!(dir.join("templates/new-template").exists());
        assert!(dir.join("templates/new-template/template.yml").exists());

        assert_eq!(
            template,
            Template {
                name: "new-template".into(),
                root: dir.join("templates/new-template")
            }
        );
    }

    #[tokio::test]
    async fn creates_the_template_from_another_dir() {
        let dir = create_templates_dirs("scaffolding").await;

        let template = Generator::create(
            dir.path(),
            &GeneratorConfig {
                templates: string_vec!["./scaffolding"],
            },
        )
        .unwrap()
        .generate_template("new-template")
        .await
        .unwrap();

        assert!(dir.join("scaffolding/new-template").exists());
        assert!(dir.join("scaffolding/new-template/template.yml").exists());

        assert_eq!(
            template,
            Template {
                name: "new-template".into(),
                root: dir.join("scaffolding/new-template")
            }
        );
    }

    #[tokio::test]
    async fn cleans_and_formats_the_name() {
        let dir = create_templates_dirs("templates").await;

        let template = Generator::create(dir.path(), &GeneratorConfig::default())
            .unwrap()
            .generate_template("so&me temPlatE- with Ran!dom-Valu^es 123_")
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
                name: "sometemPlatE-withRandom-Values123_".into(),
                root: dir.join("templates/sometemPlatE-withRandom-Values123_")
            }
        );
    }
}
