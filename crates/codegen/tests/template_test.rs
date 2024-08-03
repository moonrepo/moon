use moon_codegen::{CodeGenerator, Template, TemplateContext, TemplateFile};
use moon_common::consts::CONFIG_TEMPLATE_FILENAME_YML;
use moon_common::Id;
use moon_config::{GeneratorConfig, TemplateFrontmatterConfig};
use moon_env::MoonEnvironment;
use starbase_sandbox::{create_sandbox, locate_fixture};
use std::path::PathBuf;

fn create_template_file() -> TemplateFile {
    TemplateFile::new("standard".into(), PathBuf::from("."))
}

fn create_template() -> Template {
    Template::new(Id::raw("standard"), locate_fixture("template")).unwrap()
}

fn create_context() -> TemplateContext {
    let mut context = TemplateContext::new();
    context.insert("string", "string");
    context.insert("number", &123);
    context.insert("bool", &true);
    context
}

mod template {
    use super::*;

    mod load_files {
        use super::*;

        #[test]
        fn loads_all_template_files() {
            let mut template = create_template();
            let fixture = locate_fixture("template");

            template.load_files(&fixture, &create_context()).unwrap();

            // Skips partials
            assert_eq!(
                template
                    .files
                    .into_values()
                    .map(|f| f.source_path)
                    .collect::<Vec<_>>(),
                vec![
                    fixture.join("file.raw.txt"),
                    fixture.join("file.ts"),
                    fixture.join("file.txt"),
                    fixture.join("folder/nested-file.ts")
                ]
            );

            assert_eq!(
                template
                    .assets
                    .into_values()
                    .map(|f| f.source_path)
                    .collect::<Vec<_>>(),
                vec![fixture.join("image.jpg")]
            );
        }

        #[test]
        fn adds_all_to_tera_engine() {
            let mut template = create_template();
            let fixture = locate_fixture("template");

            template.load_files(&fixture, &create_context()).unwrap();

            let mut files = template.engine.get_template_names().collect::<Vec<_>>();
            files.sort();

            assert_eq!(
                files,
                vec![
                    "file.ts",
                    "file.txt",
                    "folder/nested-file.ts",
                    "partial-file.ts"
                ]
            );
        }

        #[test]
        fn renders_content_of_files() {
            let mut template = create_template();
            let fixture = locate_fixture("template");

            template.load_files(&fixture, &create_context()).unwrap();

            let file = template
                .files
                .values()
                .find(|f| f.name == "file.ts")
                .unwrap();

            assert_eq!(file.content, "export {};\n");
            assert_eq!(
                file.config,
                Some(TemplateFrontmatterConfig {
                    force: true,
                    ..TemplateFrontmatterConfig::default()
                })
            );

            let file = template
                .files
                .values()
                .find(|f| f.name == "file.txt")
                .unwrap();

            assert_eq!(file.content, "2\n");
            assert_eq!(file.config, None);
        }

        #[test]
        fn doesnt_render_raw_files() {
            let mut template = create_template();
            let fixture = locate_fixture("template");

            template.load_files(&fixture, &create_context()).unwrap();

            let file = template
                .files
                .values()
                .find(|f| f.name == "file.raw.txt")
                .unwrap();

            assert_eq!(file.content, "{% set my_var = 2 %}\n{{ my_var }}\n");
        }

        #[test]
        fn filters_out_schema_file() {
            let mut template = create_template();

            template
                .load_files(&locate_fixture("template"), &create_context())
                .unwrap();

            let has_schema = template
                .files
                .values()
                .any(|f| f.name.ends_with(CONFIG_TEMPLATE_FILENAME_YML));

            assert!(!has_schema);
        }

        #[tokio::test]
        async fn inherits_extended_files() {
            let sandbox = create_sandbox("generator");
            let out = sandbox.path().join("out");
            let config = GeneratorConfig::default();

            let mut codegen = CodeGenerator::new(
                sandbox.path(),
                &config,
                MoonEnvironment::new_testing(sandbox.path()).into(),
            );
            codegen.load_templates().await.unwrap();
            let mut template = codegen.get_template("extends").unwrap();

            template.load_files(&out, &create_context()).unwrap();

            // Verify sources
            assert_eq!(
                template
                    .files
                    .values()
                    .map(|f| f.source_path.clone())
                    .collect::<Vec<_>>(),
                vec![
                    sandbox.path().join("templates/extends-from-a/a.txt"),
                    sandbox.path().join("templates/extends/b.txt"), // Overwritten
                    sandbox.path().join("templates/extends/base.txt"),
                    sandbox.path().join("templates/extends-from-c/c.txt"),
                ]
            );

            // Verify dests
            assert_eq!(
                template
                    .files
                    .values()
                    .map(|f| f.dest_path.clone())
                    .collect::<Vec<_>>(),
                vec![
                    out.join("a.txt"),
                    out.join("b.txt"),
                    out.join("base.txt"),
                    out.join("c.txt"),
                ]
            );
        }
    }

    mod interpolate_path {
        use super::*;

        #[test]
        fn path_segments() {
            let mut template = create_template();
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
            let mut template = create_template();
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
            let mut template = create_template();
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
            let mut template = create_template();
            let context = create_context();

            assert_eq!(
                template
                    .interpolate_path(&PathBuf::from("folder/[unknown].ts"), &context)
                    .unwrap(),
                "folder/[unknown].ts"
            );
        }

        #[test]
        fn removes_exts() {
            let mut template = create_template();
            let context = create_context();

            assert_eq!(
                template
                    .interpolate_path(&PathBuf::from("file.ts.tera"), &context)
                    .unwrap(),
                "file.ts"
            );
            assert_eq!(
                template
                    .interpolate_path(&PathBuf::from("file.ts.twig"), &context)
                    .unwrap(),
                "file.ts"
            );
        }

        #[test]
        fn supports_filters() {
            let mut template = create_template();
            let context = create_context();

            assert_eq!(
                template
                    .interpolate_path(&PathBuf::from("folder/[string | upper_case].ts"), &context)
                    .unwrap(),
                "folder/STRING.ts"
            );
            assert_eq!(
                template
                    .interpolate_path(&PathBuf::from("folder/[bool|pascal_case].ts"), &context)
                    .unwrap(),
                "folder/True.ts"
            );
        }
    }

    mod set_content {
        use super::*;

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
            assert!(!file.config.unwrap().force);
            assert_eq!(file.content, "Content".to_owned());
        }

        #[test]
        fn skip_field() {
            let mut file = create_template_file();
            file.set_content("---\nskip: true\n---\n Content", &PathBuf::from("."))
                .unwrap();

            assert!(file.is_skipped());
            assert!(file.config.unwrap().skip);
            assert_eq!(file.content, "Content".to_owned());
        }
    }

    mod extending {
        use super::*;
        use starbase_sandbox::assert_snapshot;

        #[tokio::test]
        async fn can_include_extended_files() {
            let sandbox = create_sandbox("include");
            let out = sandbox.path().join("out");
            let config = GeneratorConfig::default();

            let mut codegen = CodeGenerator::new(
                sandbox.path(),
                &config,
                MoonEnvironment::new_testing(sandbox.path()).into(),
            );
            codegen.load_templates().await.unwrap();

            let mut template = codegen.get_template("base").unwrap();

            template.load_files(&out, &create_context()).unwrap();

            let file = template
                .files
                .values()
                .find(|f| f.name == "include.txt")
                .unwrap();

            assert_snapshot!(file.content);

            let file = template
                .files
                .values()
                .find(|f| f.name == "inheritance.txt")
                .unwrap();

            assert_snapshot!(file.content);
        }
    }
}
