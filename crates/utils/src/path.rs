pub use dirs::home_dir as get_home_dir;
use moon_error::MoonError;
use path_clean::PathClean;
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

pub fn normalize(path: &Path) -> PathBuf {
    path.to_path_buf().clean()
}

pub fn normalize_glob(path: &Path) -> String {
    // Always use forward slashes for globs
    let glob = standardize_separators(&path.to_string_lossy());

    // Remove UNC prefix as it breaks glob matching
    if std::env::consts::OS == "windows" {
        return glob.replace("//?/", "");
    }

    glob
}

#[cfg(not(windows))]
pub fn normalize_separators(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(windows)]
pub fn normalize_separators(path: &str) -> String {
    path.replace('/', "\\")
}

pub fn path_to_string(path: &Path) -> Result<String, MoonError> {
    match path.to_str() {
        Some(p) => Ok(p.to_owned()),
        None => Err(MoonError::PathInvalidUTF8(path.to_path_buf())),
    }
}

pub fn replace_home_dir(value: &str) -> String {
    if let Some(home_dir) = get_home_dir() {
        let home_dir_str = home_dir.to_str().unwrap();

        // Replace both forward and backward slashes
        return value
            .replace(home_dir_str, "~")
            .replace(&standardize_separators(home_dir_str), "~");
    }

    value.to_owned()
}

pub fn standardize_separators(path: &str) -> String {
    path.replace('\\', "/")
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
