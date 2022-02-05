use json_comments::StripComments;
use moon_error::{map_io_to_fs_error, MoonError};
use regex::Regex;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// If a file starts with "/", expand from the workspace root, otherwise the project root.
pub fn expand_root_path(file: &str, workspace_root: &Path, project_root: &Path) -> PathBuf {
    if file.starts_with('/') {
        workspace_root.join(file.strip_prefix('/').unwrap())
    } else {
        project_root.join(file)
    }
}

// This is not very exhaustive and may be inaccurate.
pub fn is_glob(value: &str) -> bool {
    let single_values = vec!['*', '?', '1'];
    let paired_values = vec![('{', '}'), ('[', ']')];
    let mut bytes = value.bytes();
    let mut is_escaped = |index: usize| {
        if index == 0 {
            return false;
        }

        bytes.nth(index - 1).unwrap_or(b' ') == b'\\'
    };

    if value.contains("**") {
        return true;
    }

    for single in single_values {
        if !value.contains(single) {
            continue;
        }

        if let Some(index) = value.find(single) {
            if !is_escaped(index) {
                return true;
            }
        }
    }

    for (open, close) in paired_values {
        if !value.contains(open) || !value.contains(close) {
            continue;
        }

        if let Some(index) = value.find(open) {
            if !is_escaped(index) {
                return true;
            }
        }
    }

    false
}

pub fn is_path_glob(path: &Path) -> bool {
    is_glob(&path.to_string_lossy())
}

pub fn read_json_file(path: &Path) -> Result<String, MoonError> {
    let handle_io_error = |e: std::io::Error| map_io_to_fs_error(e, path.to_path_buf());
    let json = fs::read_to_string(path).map_err(handle_io_error)?;

    // Remove comments
    let mut stripped = String::with_capacity(json.len());

    StripComments::new(json.as_bytes())
        .read_to_string(&mut stripped)
        .map_err(handle_io_error)?;

    // Remove trailing commas
    let stripped = Regex::new(r",(?P<valid>\s*})")
        .unwrap()
        .replace_all(&stripped, "$valid");

    Ok(String::from(stripped))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod is_glob {
        use super::*;

        #[test]
        fn returns_true_when_a_glob() {
            assert!(is_glob("**"));
            assert!(is_glob("**/src/*"));
            assert!(is_glob("src/**"));
            assert!(is_glob("*.ts"));
            assert!(is_glob("file.*"));
            assert!(is_glob("file.{js,ts}"));
            assert!(is_glob("file.[jstx]"));
            assert!(is_glob("file.tsx?"));
        }

        #[test]
        fn returns_false_when_not_glob() {
            assert!(!is_glob("dir"));
            assert!(!is_glob("file.rs"));
            assert!(!is_glob("dir/file.ts"));
            assert!(!is_glob("dir/dir/file_test.rs"));
            assert!(!is_glob("dir/dirDir/file-ts.js"));
        }

        #[test]
        fn returns_false_when_escaped_glob() {
            assert!(!is_glob("\\*.rs"));
            assert!(!is_glob("file\\?.js"));
            assert!(!is_glob("folder-\\[id\\]"));
        }
    }
}
