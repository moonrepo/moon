mod helpers;

use crate::helpers::{create_test_command, snap};

#[test]
fn no_projects() {
    let assert = create_test_command("base").arg("project-graph").assert();

    snap(&assert);
}

#[test]
fn many_projects() {
    let assert = create_test_command("projects")
        .arg("project-graph")
        .assert();

    snap(&assert);
}

#[test]
fn single_project_with_deps() {
    let assert = create_test_command("projects")
        .arg("project-graph")
        .arg("foo")
        .assert();

    snap(&assert);
}

#[test]
fn single_project_no_deps() {
    let assert = create_test_command("projects")
        .arg("project-graph")
        .arg("baz")
        .assert();

    snap(&assert);
}
