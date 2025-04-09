use moon_codegen::{MergeType, TemplateFile};
use moon_common::path::RelativePathBuf;
use moon_config::TemplateFrontmatterConfig;
use std::path::PathBuf;

mod template_file {
    use super::*;

    #[test]
    fn marks_as_raw() {
        let template = TemplateFile::new(RelativePathBuf::from("file.raw.txt"), PathBuf::new());

        assert!(template.raw);

        let template = TemplateFile::new(RelativePathBuf::from("file.raw"), PathBuf::new());

        assert!(template.raw);

        let template = TemplateFile::new(RelativePathBuf::from("file.txt.raw"), PathBuf::new());

        assert!(template.raw);
    }

    mod mergeable {
        use super::*;

        #[test]
        fn is_not_with_other_exts() {
            let template = TemplateFile::new(RelativePathBuf::from("file.txt"), PathBuf::new());

            assert_eq!(template.is_mergeable(), None);
        }

        #[test]
        fn is_with_json() {
            let template = TemplateFile::new(RelativePathBuf::from("file.json"), PathBuf::new());

            assert_eq!(template.is_mergeable(), Some(MergeType::Json));
        }

        #[test]
        fn is_with_yaml() {
            let template = TemplateFile::new(RelativePathBuf::from("file.yaml"), PathBuf::new());

            assert_eq!(template.is_mergeable(), Some(MergeType::Yaml));
        }

        #[test]
        fn uses_to_config() {
            let mut template = TemplateFile::new(RelativePathBuf::from("file.txt"), PathBuf::new());

            assert_eq!(template.is_mergeable(), None);

            template.config = Some(TemplateFrontmatterConfig {
                to: Some("file.json".into()),
                ..TemplateFrontmatterConfig::default()
            });

            assert_eq!(template.is_mergeable(), Some(MergeType::Json));
        }
    }

    mod content {
        use super::*;

        fn create_file_with_content(content: &str) -> TemplateFile {
            let mut template = TemplateFile::new(RelativePathBuf::from("file.js"), PathBuf::new());
            template
                .set_content(content, &PathBuf::from("root"))
                .unwrap();
            template
        }

        #[test]
        fn removes_raw_from_path() {
            let mut template =
                TemplateFile::new(RelativePathBuf::from("file.raw.js"), PathBuf::new());
            template
                .set_content("{{ foo }}", &PathBuf::from("root"))
                .unwrap();

            assert_eq!(template.content, "{{ foo }}");
            assert_eq!(template.config, None);
            assert_eq!(template.dest_path, PathBuf::from("root/file.js"));
        }

        #[test]
        fn sets_no_frontmatter() {
            let template = create_file_with_content("export {};");

            assert_eq!(template.content, "export {};");
            assert_eq!(template.config, None);
        }

        #[test]
        fn skips_leading_fm_block() {
            let template = create_file_with_content("---\nexport {};");

            assert_eq!(template.content, "---\nexport {};");
            assert_eq!(template.config, None);
        }

        #[test]
        fn fm_parses_skip() {
            let template = create_file_with_content("---\nskip: true\n---\nexport {};");

            assert_eq!(template.dest_path, PathBuf::from("root/file.js"));
            assert_eq!(template.content, "export {};");
            assert!(template.is_skipped());
        }

        #[test]
        fn fm_parses_force() {
            let template = create_file_with_content("---\nforce: true\n---\n\nexport {};");

            assert_eq!(template.content, "export {};");
            assert!(template.is_forced());
        }

        #[test]
        fn fm_inherits_to() {
            let template = create_file_with_content("---\nto: another/file.js\n---\n\nexport {};");

            assert_eq!(template.content, "export {};");
            assert_eq!(template.dest_path, PathBuf::from("root/another/file.js"));
        }

        #[test]
        #[should_panic(expected = "Failed to parse TemplateFrontmatterConfig")]
        fn errors_invalid_fm_syntax() {
            create_file_with_content("---\nforce = true\n---\n\nexport {};");
        }

        #[test]
        #[should_panic(expected = "unknown field `unknown`")]
        fn errors_unknown_fm_field() {
            create_file_with_content("---\nunknown: true\n---\n\nexport {};");
        }
    }
}
