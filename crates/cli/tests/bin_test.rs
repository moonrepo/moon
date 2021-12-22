mod helpers;

use crate::helpers::create_test_command;
use predicates::prelude::*;

#[test]
fn invalid_tool() {
    let assert = create_test_command("base")
        .arg("bin")
        .arg("unknown")
        .assert();

    assert
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("\"unknown\" isn\'t a valid value"));
}

#[test]
fn not_configured() {
    let assert = create_test_command("base").arg("bin").arg("yarn").assert();

    assert.failure().code(1).stdout("");
}

#[test]
fn not_installed() {
    let assert = create_test_command("base").arg("bin").arg("node").assert();

    assert.failure().code(2).stdout("");
}
