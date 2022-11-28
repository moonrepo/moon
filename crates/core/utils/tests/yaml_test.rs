use moon_test_utils::{assert_snapshot, create_sandbox};
use moon_utils::yaml;
use std::fs;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;

mod editor_config {
    use super::*;

    pub fn append_editor_config(root: &Path, data: &str) {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(root.join(".editorconfig"))
            .unwrap();

        writeln!(file, "\n\n{}", data).unwrap();
    }

    #[test]
    fn uses_defaults_when_no_config() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.yaml");

        yaml::write_with_config(&path, yaml::read(&path).unwrap()).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn can_change_space_indent() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.yaml");

        append_editor_config(sandbox.path(), "[*.yaml]\nindent_size = 8");

        yaml::write_with_config(&path, yaml::read(&path).unwrap()).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn can_change_tab_indent() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.yaml");

        append_editor_config(sandbox.path(), "[*.yaml]\nindent_style = tab");

        yaml::write_with_config(&path, yaml::read(&path).unwrap()).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn can_enable_trailing_line() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.yaml");

        append_editor_config(sandbox.path(), "[*.yaml]\ninsert_final_newline = true");

        yaml::write_with_config(&path, yaml::read(&path).unwrap()).unwrap();

        assert!(fs::read_to_string(&path).unwrap().ends_with('\n'));
    }

    #[test]
    fn can_disable_trailing_line() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.yaml");

        append_editor_config(sandbox.path(), "[*.yaml]\ninsert_final_newline = false");

        yaml::write_with_config(&path, yaml::read(&path).unwrap()).unwrap();

        assert!(!fs::read_to_string(&path).unwrap().ends_with('\n'));
    }
}
