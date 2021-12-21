mod helpers;

use crate::helpers::{create_test_command, snap};

#[test]
fn empty_config() {
    let assert = create_test_command("projects")
        .arg("project")
        .arg("emptyConfig")
        .assert();

    snap(&assert);
}

#[test]
fn no_config() {
    let assert = create_test_command("projects")
        .arg("project")
        .arg("noConfig")
        .assert();

    snap(&assert);
}

#[test]
fn basic_config() {
    // with dependsOn and fileGroups
    let assert = create_test_command("projects")
        .arg("project")
        .arg("basic")
        .assert();

    snap(&assert);
}

#[test]
fn advanced_config() {
    // with project metadata
    let assert = create_test_command("projects")
        .arg("project")
        .arg("advanced")
        .assert();

    snap(&assert);
}

#[test]
fn depends_on_paths() {
    // shows dependsOn paths when they exist
    let assert = create_test_command("projects")
        .arg("project")
        .arg("foo")
        .assert();

    snap(&assert);
}
