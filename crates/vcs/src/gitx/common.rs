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
