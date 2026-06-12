use super::git_error::GitError;
use regex::Regex;
use std::sync::LazyLock;

pub static STATUS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(M|T|A|D|R|C|U|\?|!| )(M|T|A|D|R|C|U|\?|!| ) ").unwrap());

pub static DIFF_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(A|D|M|T|U|X)$").unwrap());

pub static DIFF_SCORE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(C|M|R)(\d{3})$").unwrap());

pub static VERSION_CLEAN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.(windows|win|msysgit|msys|vfs)(\.\d+){1,2}").unwrap());

/// Validate that a revision (typically user provided) doesn't look like a
/// command line option, otherwise it can be abused for argument injection,
/// like `--output=file`. Valid revisions can never start with a dash.
pub fn validate_revision(revision: &str) -> Result<(), GitError> {
    if revision.starts_with('-') {
        return Err(GitError::InvalidRevision {
            revision: revision.to_owned(),
        });
    }

    Ok(())
}

pub fn clean_git_version(version: String) -> String {
    let version = if let Some(index) = version.find('(') {
        &version[0..index]
    } else {
        &version
    };

    let version = version
        .to_lowercase()
        .replace("git", "")
        .replace("version", "")
        .replace("for windows", "")
        .replace("(32-bit)", "")
        .replace("(64-bit)", "")
        .replace("(32bit)", "")
        .replace("(64bit)", "");

    let version = VERSION_CLEAN.replace(&version, "");

    // Some older versions have more than 3 numbers,
    // so ignore any non major, minor, or patches
    let mut parts = version.trim().split('.');

    format!(
        "{}.{}.{}",
        parts.next().unwrap_or("0"),
        parts.next().unwrap_or("0"),
        parts.next().unwrap_or("0")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix() {
        assert_eq!(clean_git_version("git version 1.2.3".into()), "1.2.3");
        assert_eq!(clean_git_version(" git version 1.2.3".into()), "1.2.3");
        assert_eq!(clean_git_version("git version 1.2.3 ".into()), "1.2.3");
        assert_eq!(clean_git_version(" git version 1.2.3 ".into()), "1.2.3");
        assert_eq!(
            clean_git_version("git version 1.2.3 (64-bit)".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version("git version 1.2.3 (32bit)".into()),
            "1.2.3"
        );
    }

    #[test]
    fn macos() {
        assert_eq!(
            clean_git_version("git version 1.2.3 (Apple Git-55)".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version("git version 2.15.1 (Apple Git-101)".into()),
            "2.15.1"
        );
    }

    #[test]
    fn windows() {
        assert_eq!(
            clean_git_version("git version 1.2.3.windows.1".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version(" git for windows 1.2.3.windows.0".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version("git version 1.2.3.windows.10 (32-Bit)  ".into()),
            "1.2.3"
        );

        assert_eq!(
            clean_git_version("  git for windows 1.2.3.win.1".into()),
            "1.2.3"
        );
        assert_eq!(clean_git_version("git 1.2.3.msysgit.1".into()), "1.2.3");
        assert_eq!(
            clean_git_version(" git version 1.2.3.msysgit.11 ".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version("git for windows 1.2.3.msysgit.23  (64bit) ".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version("git version 1.2.3.vfs.0.0".into()),
            "1.2.3"
        );
    }

    #[test]
    fn other() {
        assert_eq!(clean_git_version("git version 1.8.3.1".into()), "1.8.3");
    }

    #[test]
    fn revisions() {
        assert!(validate_revision("master").is_ok());
        assert!(validate_revision("HEAD~1").is_ok());
        assert!(validate_revision("v1.2.3").is_ok());
        assert!(validate_revision("a1b2c3d").is_ok());
        assert!(validate_revision("").is_ok());

        assert!(validate_revision("-x").is_err());
        assert!(validate_revision("--output=file").is_err());
    }
}
