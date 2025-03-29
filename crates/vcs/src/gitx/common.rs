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
