use insta::assert_snapshot;
use moon_utils::json;
use moon_utils::test::create_sandbox;
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
        let fixture = create_sandbox("editor-config");
        let path = fixture.path().join("file.json");

        json::write_with_config(&path, json::read(&path).unwrap(), true).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn writes_ugly() {
        let fixture = create_sandbox("editor-config");
        let path = fixture.path().join("file.json");

        json::write_with_config(&path, json::read(&path).unwrap(), false).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn can_change_space_indent() {
        let fixture = create_sandbox("editor-config");
        let path = fixture.path().join("file.json");

        append_editor_config(fixture.path(), "[*.json]\nindent_size = 8");

        json::write_with_config(&path, json::read(&path).unwrap(), true).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn can_change_tab_indent() {
        let fixture = create_sandbox("editor-config");
        let path = fixture.path().join("file.json");

        append_editor_config(fixture.path(), "[*.json]\nindent_style = tab");

        json::write_with_config(&path, json::read(&path).unwrap(), true).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn can_enable_trailing_line() {
        let fixture = create_sandbox("editor-config");
        let path = fixture.path().join("file.json");

        append_editor_config(fixture.path(), "[*.json]\ninsert_final_newline = true");

        json::write_with_config(&path, json::read(&path).unwrap(), true).unwrap();

        assert!(fs::read_to_string(&path).unwrap().ends_with('\n'));
    }

    #[test]
    fn can_disable_trailing_line() {
        let fixture = create_sandbox("editor-config");
        let path = fixture.path().join("file.json");

        append_editor_config(fixture.path(), "[*.json]\ninsert_final_newline = false");

        json::write_with_config(&path, json::read(&path).unwrap(), true).unwrap();

        assert!(!fs::read_to_string(&path).unwrap().ends_with('\n'));
    }
}
