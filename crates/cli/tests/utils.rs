#![allow(dead_code)]

use moon_test_utils::{assert_cmd::assert::Assert, get_assert_output};
use moon_utils::path;
use std::path::Path;

pub fn replace_fixtures_dir<T: AsRef<str>, P: AsRef<Path>>(value: T, dir: P) -> String {
    let dir_str = dir.as_ref().to_str().unwrap();

    // Replace both forward and backward slashes
    value
        .as_ref()
        .replace(dir_str, "<WORKSPACE>")
        .replace(&path::standardize_separators(dir_str), "<WORKSPACE>")
}

pub fn get_path_safe_output(assert: &Assert, fixtures_dir: &Path) -> String {
    let result = path::replace_home_dir(replace_fixtures_dir(
        get_assert_output(assert),
        fixtures_dir,
    ));

    result.replace("/private<", "<")
}
