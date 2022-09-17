use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_generator::{FileState, Template, TemplateContext, TemplateFile};
use moon_utils::test::get_fixtures_dir;
use std::path::{Path, PathBuf};

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
        let dest = assert_fs::TempDir::new().unwrap();
        let mut template = create_template();

        template
            .load_files(dest.path(), &create_context())
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

// mod template_files {
//     use super::*;

//     fn new_file(dest: &Path) -> TemplateFile {
//         TemplateFile {
//             config: None,
//             content: String::new(),
//             dest_path: dest.join("folder/nested-file.ts"),
//             name: "folder/nested-file.ts".into(),
//             overwrite: false,
//             source_path: get_fixtures_dir("generator")
//                 .join("templates/standard/folder/nested-file.ts"),
//             state: FileState::Created,
//         }
//     }

//     #[tokio::test]
//     async fn creates_file() {
//         let dest = assert_fs::TempDir::new().unwrap();
//         let file = new_file(dest.path());

//         assert!(file.should_write());
//         assert_eq!(file.state(), FileState::Created);
//     }

//     #[tokio::test]
//     async fn overwrites_existing_file() {
//         let dest = assert_fs::TempDir::new().unwrap();
//         let mut file = new_file(dest.path());
//         file.existed = true;
//         file.overwrite = true;

//         assert!(file.should_write());
//         assert_eq!(file.state(), FileState::Replaced);
//     }

//     #[tokio::test]
//     async fn doesnt_overwrite_existing_file() {
//         let dest = assert_fs::TempDir::new().unwrap();
//         let mut file = new_file(dest.path());
//         file.existed = true;
//         file.overwrite = false;

//         assert!(!file.should_write());
//         assert_eq!(file.state(), FileState::Skipped);
//     }
// }
