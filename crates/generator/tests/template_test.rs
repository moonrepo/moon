use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_generator::{FileState, Template, TemplateFile};
use moon_utils::test::get_fixtures_dir;
use std::path::Path;

mod load_files {
    use super::*;

    #[tokio::test]
    async fn filters_out_schema_file() {
        let dest = assert_fs::TempDir::new().unwrap();

        let mut template = Template::new(
            "standard".into(),
            get_fixtures_dir("generator").join("templates/standard"),
        )
        .unwrap();

        template.load_files(dest.path()).await.unwrap();

        let has_schema = template
            .files
            .iter()
            .any(|f| f.name.ends_with(CONFIG_TEMPLATE_FILENAME));

        assert!(!has_schema);
    }
}

mod template_files {
    use super::*;

    fn new_file(dest: &Path) -> TemplateFile {
        TemplateFile {
            dest_path: dest.join("folder/nested-file.ts"),
            existed: false,
            name: "folder/nested-file.ts".into(),
            overwrite: false,
            source_path: get_fixtures_dir("generator")
                .join("templates/standard/folder/nested-file.ts"),
        }
    }

    #[tokio::test]
    async fn creates_file() {
        let dest = assert_fs::TempDir::new().unwrap();
        let file = new_file(dest.path());

        file.generate().await.unwrap();

        assert!(file.dest_path.exists());
        assert_eq!(file.state(), FileState::Created);
    }

    #[tokio::test]
    async fn overwrites_existing_file() {
        let dest = assert_fs::TempDir::new().unwrap();
        let mut file = new_file(dest.path());
        file.existed = true;
        file.overwrite = true;

        file.generate().await.unwrap();

        assert!(file.dest_path.exists());
        assert_eq!(file.state(), FileState::Replaced);
    }

    #[tokio::test]
    async fn doesnt_overwrite_existing_file() {
        let dest = assert_fs::TempDir::new().unwrap();
        let mut file = new_file(dest.path());
        file.existed = true;
        file.overwrite = false;

        file.generate().await.unwrap();

        assert!(!file.dest_path.exists());
        assert_eq!(file.state(), FileState::Skipped);
    }
}
