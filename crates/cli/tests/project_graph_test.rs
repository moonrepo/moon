mod helpers;
use insta::assert_snapshot;

use crate::helpers::{create_test_command, get_assert_output};

#[test]
fn no_projects() {
    let assert = create_test_command("base").arg("project-graph").assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn many_projects() {
    let assert = create_test_command("projects")
        .arg("project-graph")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn single_project_with_deps() {
    let assert = create_test_command("projects")
        .arg("project-graph")
        .arg("foo")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn single_project_no_deps() {
    let assert = create_test_command("projects")
        .arg("project-graph")
        .arg("baz")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}
