use moon_config::{LanguageType, PlatformType};
use moon_project_builder::ProjectBuilder;
use starbase_sandbox::create_sandbox;

mod project_builder {
    use super::*;

    #[test]
    #[should_panic(expected = "MissingProjectAtSource(\"qux\")")]
    fn errors_missing_source() {
        let sandbox = create_sandbox("builder");

        ProjectBuilder::new("qux".into(), "qux".into(), sandbox.path()).unwrap();
    }

    mod language_detect {
        use super::*;

        #[test]
        fn inherits_from_config() {
            let sandbox = create_sandbox("builder");

            let mut builder =
                ProjectBuilder::new("bar".into(), "bar".into(), sandbox.path()).unwrap();
            builder
                .load_local_config(|_| LanguageType::Unknown)
                .unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.language, LanguageType::Rust);
        }

        #[test]
        fn detects_from_env() {
            let sandbox = create_sandbox("builder");

            let mut builder =
                ProjectBuilder::new("foo".into(), "foo".into(), sandbox.path()).unwrap();
            builder
                .load_local_config(|_| LanguageType::TypeScript)
                .unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.language, LanguageType::TypeScript);
        }
    }

    mod platform_detect {
        use super::*;

        #[test]
        fn inherits_from_config() {
            let sandbox = create_sandbox("builder");

            let mut builder =
                ProjectBuilder::new("baz".into(), "baz".into(), sandbox.path()).unwrap();
            builder
                .load_local_config(|_| LanguageType::Unknown)
                .unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.platform, PlatformType::Node);
        }

        #[test]
        fn infers_from_config_lang() {
            let sandbox = create_sandbox("builder");

            let mut builder =
                ProjectBuilder::new("bar".into(), "bar".into(), sandbox.path()).unwrap();
            builder
                .load_local_config(|_| LanguageType::Unknown)
                .unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.platform, PlatformType::Rust);
        }

        #[test]
        fn infers_from_detected_lang() {
            let sandbox = create_sandbox("builder");

            let mut builder =
                ProjectBuilder::new("foo".into(), "foo".into(), sandbox.path()).unwrap();
            builder
                .load_local_config(|_| LanguageType::TypeScript)
                .unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.platform, PlatformType::Node);
        }
    }
}
