use moon_config::GeneratorConfig;
use moon_generator::Generator;
use moon_test_utils::create_sandbox;
use moon_utils::string_vec;

mod create_template {
    use super::*;

    #[test]
    #[should_panic(expected = "ExistingTemplate(\"standard\"")]
    fn errors_if_already_exists() {
        let sandbox = create_sandbox("generator");

        Generator::load(sandbox.path(), &GeneratorConfig::default())
            .unwrap()
            .create_template("standard")
            .unwrap();
    }

    #[test]
    fn creates_the_template() {
        let sandbox = create_sandbox("generator");

        let template = Generator::load(sandbox.path(), &GeneratorConfig::default())
            .unwrap()
            .create_template("new-template")
            .unwrap();

        assert!(sandbox.path().join("templates/new-template").exists());
        assert!(sandbox
            .path()
            .join("templates/new-template/template.yml")
            .exists());

        assert_eq!(template.name, "new-template".to_owned());
        assert_eq!(template.root, sandbox.path().join("templates/new-template"));
    }

    #[test]
    fn creates_the_template_from_another_dir() {
        let sandbox = create_sandbox("generator");

        let template = Generator::load(
            sandbox.path(),
            &GeneratorConfig {
                templates: string_vec!["./scaffolding"],
            },
        )
        .unwrap()
        .create_template("new-template")
        .unwrap();

        assert!(sandbox.path().join("scaffolding/new-template").exists());
        assert!(sandbox
            .path()
            .join("scaffolding/new-template/template.yml")
            .exists());

        assert_eq!(template.name, "new-template".to_owned());
        assert_eq!(
            template.root,
            sandbox.path().join("scaffolding/new-template")
        );
    }

    #[test]
    fn cleans_and_formats_the_name() {
        let sandbox = create_sandbox("generator");

        let template = Generator::load(sandbox.path(), &GeneratorConfig::default())
            .unwrap()
            .create_template("so&me temPlatE- with Ran!dom-Valu^es 123_")
            .unwrap();

        assert!(sandbox
            .path()
            .join("templates/so-me-temPlatE--with-Ran-dom-Valu-es-123_")
            .exists());
        assert!(sandbox
            .path()
            .join("templates/so-me-temPlatE--with-Ran-dom-Valu-es-123_/template.yml")
            .exists());

        assert_eq!(
            template.name,
            "so-me-temPlatE--with-Ran-dom-Valu-es-123_".to_owned()
        );
        assert_eq!(
            template.root,
            sandbox
                .path()
                .join("templates/so-me-temPlatE--with-Ran-dom-Valu-es-123_")
        );
    }
}
