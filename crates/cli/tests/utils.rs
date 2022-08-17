#![allow(dead_code)]

use moon_utils::path::replace_home_dir;
use moon_utils::test::{get_assert_output, replace_fixtures_dir};
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

pub fn update_version_workspace_config(dir: &Path, old_version: &str, new_version: &str) {
    let mut config = fs::read_to_string(dir.join(".moon/workspace.yml")).unwrap();

    config = config.replace(old_version, new_version);

    fs::write(dir.join(".moon/workspace.yml"), config).unwrap();
}

pub fn get_path_safe_output(assert: &assert_cmd::assert::Assert, fixtures_dir: &Path) -> String {
    let result = replace_home_dir(replace_fixtures_dir(
        get_assert_output(assert),
        fixtures_dir,
    ));

    result.replace("/private<", "<")
}
