use crate::path::{path_to_string, standardize_separators};
use moon_error::MoonError;
use std::path::{Path, PathBuf};
use wax::{Any, Pattern};

pub use wax::Glob;

pub struct GlobSet<'t> {
    any: Any<'t>,
    // globs: Vec<Glob<'t>>,
}

impl<'t> GlobSet<'t> {
    pub fn new(patterns: &'t [String]) -> Self {
        let globs = patterns
            .iter()
            .map(|p| Glob::new(p).unwrap())
            .collect::<Vec<Glob>>();

        GlobSet {
            any: wax::any::<Glob, _>(globs).unwrap(),
            // globs,
        }
    }

    // #[cfg(not(windows))]
    pub fn is_match(&self, path: &Path) -> bool {
        self.any.is_match(path)
    }

    // #[cfg(windows)]
    // pub fn is_match(&self, path: &Path) -> bool {
    //     let path = PathBuf::from(normalize(path).unwrap());

    //     self.any.is_match(path)
    // }
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

pub fn normalize(path: &Path) -> Result<String, MoonError> {
    // Always use forward slashes for globs
    let glob = standardize_separators(&path_to_string(path)?);

    // Remove UNC prefix as it breaks glob matching
    if cfg!(windows) {
        return Ok(glob.replace("//?/", ""));
    }

    Ok(glob)
}

/// Wax currently doesn't support negated globs (starts with !),
/// so we must extract them manually.
pub fn split_patterns(patterns: &[String]) -> (Vec<String>, Vec<String>) {
    let mut expressions = vec![];
    let mut negations = vec![];

    for pattern in patterns {
        if pattern.starts_with('!') {
            negations.push(pattern.strip_prefix('!').unwrap().to_owned());
        } else if pattern.starts_with('/') {
            expressions.push(pattern.strip_prefix('/').unwrap().to_owned());
        } else {
            expressions.push(pattern.clone());
        }
    }

    (expressions, negations)
}

pub fn walk(base_dir: &Path, patterns: &[String]) -> Vec<PathBuf> {
    let (expressions, _negations) = split_patterns(patterns);
    let mut paths = vec![];

    for expression in expressions {
        let glob = Glob::new(&expression).unwrap();

        for entry in glob.walk(base_dir, usize::MAX)
        // .not(&negations)
        {
            match entry {
                Ok(e) => paths.push(e.into_path()),
                Err(_) => {
                    // Will crash if the file doesnt exist
                    continue;
                }
            };
        }
    }

    paths
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
