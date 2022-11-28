use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_generator::{Template, TemplateContext, TemplateFile};
use moon_utils::test::get_fixtures_dir;
use std::path::PathBuf;

fn create_template_file() -> TemplateFile {
    TemplateFile::load("standard".into(), PathBuf::from("."))
}

fn create_template() -> Template {
    Template::new(
        "standard".into(),
        get_fixtures_dir("generator").join("templates/standard"),
    )
    .unwrap()
}

fn create_context() -> TemplateContext {
    let mut context = TemplateContext::new();
    context.insert("string", "string");
    context.insert("number", &123);
    context.insert("bool", &true);
    context
}

mod load_files {
    use super::*;

    #[tokio::test]
    async fn filters_out_schema_file() {
        let mut template = create_template();

        template
            .load_files(&get_fixtures_dir("generator"), &create_context())
            .await
            .unwrap();

        let has_schema = template
            .files
            .iter()
            .any(|f| f.name.ends_with(CONFIG_TEMPLATE_FILENAME));

        assert!(!has_schema);
    }
}

mod interpolate_path {
    use super::*;

    #[test]
    fn path_segments() {
        let template = create_template();
        let context = create_context();

        assert_eq!(
            template
                .interpolate_path(&PathBuf::from("folder/[string].ts"), &context)
                .unwrap(),
            "folder/string.ts"
        );
        assert_eq!(
            template
                .interpolate_path(&PathBuf::from("[number]/file.ts"), &context)
                .unwrap(),
            "123/file.ts"
        );
        assert_eq!(
            template
                .interpolate_path(&PathBuf::from("[bool]"), &context)
                .unwrap(),
            "true"
        );
    }

    #[test]
    fn var_casing() {
        let template = create_template();
        let mut context = create_context();
        context.insert("camelCase", "camelCase");
        context.insert("PascalCase", "PascalCase");
        context.insert("snake_case", "snake_case");

        assert_eq!(
            template
                .interpolate_path(&PathBuf::from("folder/[camelCase]/file.ts"), &context)
                .unwrap(),
            "folder/camelCase/file.ts"
        );
        assert_eq!(
            template
                .interpolate_path(&PathBuf::from("folder/[PascalCase]/file.ts"), &context)
                .unwrap(),
            "folder/PascalCase/file.ts"
        );
        assert_eq!(
            template
                .interpolate_path(&PathBuf::from("folder/[snake_case]/file.ts"), &context)
                .unwrap(),
            "folder/snake_case/file.ts"
        );
    }

    #[test]
    fn multiple_vars() {
        let template = create_template();
        let context = create_context();

        assert_eq!(
            template
                .interpolate_path(&PathBuf::from("folder/[string]-[number].ts"), &context)
                .unwrap(),
            "folder/string-123.ts"
        );
        assert_eq!(
            template
                .interpolate_path(&PathBuf::from("folder/[string][number].ts"), &context)
                .unwrap(),
            "folder/string123.ts"
        );
    }

    #[test]
    fn ignores_unknown_vars() {
        let template = create_template();
        let context = create_context();

        assert_eq!(
            template
                .interpolate_path(&PathBuf::from("folder/[unknown].ts"), &context)
                .unwrap(),
            "folder/[unknown].ts"
        );
    }
}

mod set_content {
    use super::*;
    use moon_config::TemplateFrontmatterConfig;

    #[test]
    fn works_without_frontmatter() {
        let mut file = create_template_file();
        file.set_content("Content", &PathBuf::from(".")).unwrap();

        assert_eq!(file.config, None);
        assert_eq!(file.content, "Content".to_owned());
    }

    #[test]
    fn works_with_empty_frontmatter() {
        let mut file = create_template_file();
        file.set_content("---\n---\nContent", &PathBuf::from("."))
            .unwrap();

        assert_eq!(file.config, Some(TemplateFrontmatterConfig::default()));
        assert_eq!(file.content, "Content".to_owned());
    }

    #[test]
    fn to_field() {
        let mut file = create_template_file();
        file.set_content("---\nto: some/path.txt\n---\n Content", &PathBuf::from("."))
            .unwrap();

        assert_eq!(file.config.unwrap().to, Some("some/path.txt".into()));
        assert_eq!(file.content, "Content".to_owned());
    }

    #[test]
    fn to_joins_with_dest() {
        let mut file = create_template_file();
        file.set_content(
            "---\nto: some/path.txt\n---\n  Content",
            &PathBuf::from("/foo"),
        )
        .unwrap();

        assert_eq!(file.dest_path, PathBuf::from("/foo/some/path.txt"));
        assert_eq!(file.content, "Content".to_owned());
    }

    #[test]
    fn force_field() {
        let mut file = create_template_file();
        file.set_content("---\nforce: false\n---\nContent", &PathBuf::from("."))
            .unwrap();

        assert!(!file.is_forced());
        assert_eq!(file.config.unwrap().force, Some(false));
        assert_eq!(file.content, "Content".to_owned());
    }

    #[test]
    fn skip_field() {
        let mut file = create_template_file();
        file.set_content("---\nskip: true\n---\n Content", &PathBuf::from("."))
            .unwrap();

        assert!(file.is_skipped());
        assert_eq!(file.config.unwrap().skip, Some(true));
        assert_eq!(file.content, "Content".to_owned());
    }
}
