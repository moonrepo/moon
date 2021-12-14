mod helpers;

use crate::helpers::create_test_command;
use insta::assert_snapshot;

#[test]
fn empty_config() {
    let assert = create_test_command("projects")
        .arg("project")
        .arg("emptyConfig")
        .assert();

    assert_snapshot!(String::from_utf8(assert.get_output().stdout.to_owned()).unwrap());
}

#[test]
fn no_config() {
    let assert = create_test_command("projects")
        .arg("project")
        .arg("noConfig")
        .assert();

    assert_snapshot!(String::from_utf8(assert.get_output().stdout.to_owned()).unwrap());
}
