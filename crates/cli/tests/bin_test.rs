use moon_utils::test::create_moon_command;
use predicates::prelude::*;

// This requires installing the toolchain which is quite heavy in tests!
// #[test]
// fn valid_tool() {
//     let assert = create_moon_command("cases").arg("bin").arg("node").assert();

//     assert
//         .success()
//         .code(0)
//         .stdout("")
//         .stderr(predicate::str::contains("\"unknown\" isn\'t a valid value"));
// }

#[test]
fn invalid_tool() {
    let assert = create_moon_command("cases")
        .arg("bin")
        .arg("unknown")
        .assert();

    assert
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("\"unknown\" isn\'t a valid value"));
}

// We use a different Node.js version as to not conflict with other tests!

#[test]
fn not_configured() {
    let assert = create_moon_command("cases")
        .arg("bin")
        .arg("yarn")
        .env("MOON_NODE_VERSION", "16.2.0")
        .assert();

    assert.failure().code(1).stdout("");
}

#[test]
fn not_installed() {
    let assert = create_moon_command("cases")
        .arg("bin")
        .arg("node")
        .env("MOON_NODE_VERSION", "16.2.0")
        .assert();

    assert.failure().code(2).stdout("");
}
