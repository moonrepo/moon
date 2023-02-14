use moon_test_utils::{assert_snapshot, create_sandbox};
use moon_utils::json;
use std::fs;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;

mod merge {
    use super::*;
    use moon_utils::json::json as object;

    #[test]
    pub fn merges_fields() {
        let prev = object!({
            "base": null,
            "str": "abc",
            "num": 123,
            "bool": true,
            "arr": [1, 2, 3],
            "obj": {
                "key": 123,
                "nested": {
                    "key2": "abc",
                },
            },
        });
        let next = object!({
            "base": {},
            "str": "xyz",
            "arr": [1, 2, 3, 4, 5, 6],
            "obj": {
                "key": null,
                "sub": {
                    "key3": false
                }
            },
        });

        assert_eq!(
            json::merge(&prev, &next),
            object!({
                "base": {},
                "str": "xyz",
                "num": 123,
                "bool": true,
                "arr": [1, 2, 3, 4, 5, 6],
                "obj": {
                    "key": null,
                    "nested": {
                        "key2": "abc",
                    },
                    "sub": {
                        "key3": false
                    }
                },
            })
        );
    }
}

mod editor_config {
    use super::*;

    pub fn append_editor_config(root: &Path, data: &str) {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(root.join(".editorconfig"))
            .unwrap();

        writeln!(file, "\n\n{data}").unwrap();
    }

    #[test]
    fn uses_defaults_when_no_config() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.json");

        json::write_with_config(&path, json::read(&path).unwrap(), true).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn writes_ugly() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.json");

        json::write_with_config(&path, json::read(&path).unwrap(), false).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn can_change_space_indent() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.json");

        append_editor_config(sandbox.path(), "[*.json]\nindent_size = 8");

        json::write_with_config(&path, json::read(&path).unwrap(), true).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn can_change_tab_indent() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.json");

        append_editor_config(sandbox.path(), "[*.json]\nindent_style = tab");

        json::write_with_config(&path, json::read(&path).unwrap(), true).unwrap();

        assert_snapshot!(fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn can_enable_trailing_line() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.json");

        append_editor_config(sandbox.path(), "[*.json]\ninsert_final_newline = true");

        json::write_with_config(&path, json::read(&path).unwrap(), true).unwrap();

        assert!(fs::read_to_string(&path).unwrap().ends_with('\n'));
    }

    #[test]
    fn can_disable_trailing_line() {
        let sandbox = create_sandbox("editor-config");
        let path = sandbox.path().join("file.json");

        append_editor_config(sandbox.path(), "[*.json]\ninsert_final_newline = false");

        json::write_with_config(&path, json::read(&path).unwrap(), true).unwrap();

        assert!(!fs::read_to_string(&path).unwrap().ends_with('\n'));
    }
}
