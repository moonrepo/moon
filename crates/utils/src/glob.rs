use crate::path;
use lazy_static::lazy_static;
use moon_error::MoonError;
use regex::Regex;
use std::path::{Path, PathBuf};
pub use wax::Glob;
use wax::{Any, GlobError as WaxGlobError, LinkBehavior, Negation, Pattern};

lazy_static! {
    pub static ref WINDOWS_PREFIX: Regex = Regex::new(r"^(//\?/)?[A-Z]:").unwrap();
}

pub type GlobError = WaxGlobError<'static>;

pub struct GlobSet<'t> {
    any: Any<'t>,
}

impl<'t> GlobSet<'t> {
    #[track_caller]
    pub fn new(patterns: Vec<String>) -> Result<Self, GlobError> {
        let mut globs = vec![];

        for pattern in &patterns {
            globs.push(create_glob(pattern)?.into_owned());
        }

        Ok(GlobSet {
            any: wax::any::<Glob, _>(globs).unwrap(),
        })
    }

    pub fn matches(&self, path: &Path) -> Result<bool, MoonError> {
        Ok(self.any.is_match(path))
    }
}

pub fn create_glob(pattern: &str) -> Result<Glob, GlobError> {
    Ok(Glob::new(pattern).map_err(|e| e.into_owned())?)
}

// This is not very exhaustive and may be inaccurate.
pub fn is_glob<T: AsRef<str>>(value: T) -> bool {
    let value = value.as_ref();
    let single_values = vec!['*', '?', '!'];
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

pub fn normalize<T: AsRef<Path>>(path: T) -> Result<String, MoonError> {
    path::to_virtual_string(path.as_ref())
}

pub fn remove_drive_prefix<T: AsRef<str>>(glob: T) -> String {
    WINDOWS_PREFIX.replace_all(glob.as_ref(), "**").to_string()
}

/// Wax currently doesn't support negated globs (starts with !),
/// so we must extract them manually.
#[track_caller]
pub fn split_patterns<P: AsRef<str>>(patterns: &[P]) -> Result<(Vec<Glob>, Vec<Glob>), GlobError> {
    let mut expressions = vec![];
    let mut negations = vec![];

    for pattern in patterns {
        let pattern = pattern.as_ref();

        if pattern.starts_with('!') {
            negations.push(create_glob(pattern.strip_prefix('!').unwrap())?);
        } else if pattern.starts_with('/') {
            expressions.push(create_glob(pattern.strip_prefix('/').unwrap())?);
        } else {
            expressions.push(create_glob(pattern)?);
        }
    }

    // Always ignore common directories
    negations.push(create_glob("**/.*/**")?); // .git, .moon, .yarn, etc
    negations.push(create_glob("**/node_modules/**")?);

    Ok((expressions, negations))
}

#[track_caller]
pub fn walk<T: AsRef<Path>, P: AsRef<str>>(
    base_dir: T,
    patterns: &[P],
) -> Result<Vec<PathBuf>, GlobError> {
    let (globs, negations) = split_patterns(patterns)?;
    let negation = Negation::try_from_patterns(negations).unwrap();
    let mut paths = vec![];

    for glob in globs {
        for entry in glob.walk_with_behavior(base_dir.as_ref(), LinkBehavior::ReadFile) {
            match entry {
                Ok(e) => {
                    // Filter out negated results
                    if negation.target(&e).is_some() {
                        continue;
                    }

                    paths.push(e.into_path());
                }
                Err(_) => {
                    // Will crash if the file doesnt exist
                    continue;
                }
            };
        }
    }

    Ok(paths)
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

    mod windows_prefix {
        use super::*;

        #[test]
        fn removes_unc_and_drive_prefix() {
            assert_eq!(
                WINDOWS_PREFIX
                    .replace_all("//?/D:/Projects/moon", "**")
                    .to_string(),
                String::from("**/Projects/moon")
            );
        }
    }
}
