#![allow(dead_code)]

use moon_test_utils::get_assert_output;
use moon_utils::path;
use std::fs;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;

pub fn append_workspace_config(root: &Path, yaml: &str) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(root.join(".moon/workspace.yml"))
        .unwrap();

    writeln!(file, "{}", yaml).unwrap();
}

pub fn append_toolchain_config(root: &Path, yaml: &str) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(root.join(".moon/toolchain.yml"))
        .unwrap();

    writeln!(file, "{}", yaml).unwrap();
}

pub fn update_toolchain_config(dir: &Path, old: &str, new: &str) {
    let mut config = fs::read_to_string(dir.join(".moon/toolchain.yml")).unwrap();

    config = config.replace(old, new);

    fs::write(dir.join(".moon/toolchain.yml"), config).unwrap();
}

pub fn replace_fixtures_dir<T: AsRef<str>, P: AsRef<Path>>(value: T, dir: P) -> String {
    let dir_str = dir.as_ref().to_str().unwrap();

    // Replace both forward and backward slashes
    value
        .as_ref()
        .replace(dir_str, "<WORKSPACE>")
        .replace(&path::standardize_separators(dir_str), "<WORKSPACE>")
}

pub fn get_path_safe_output(assert: &assert_cmd::assert::Assert, fixtures_dir: &Path) -> String {
    let result = path::replace_home_dir(replace_fixtures_dir(
        get_assert_output(assert),
        fixtures_dir,
    ));

    result.replace("/private<", "<")
}
