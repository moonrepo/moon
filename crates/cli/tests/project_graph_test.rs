use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, get_assert_output};

#[test]
fn no_projects() {
    let assert = create_moon_command("base").arg("project-graph").assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn many_projects() {
    let assert = create_moon_command("projects")
        .arg("project-graph")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn single_project_with_dependencies() {
    let assert = create_moon_command("projects")
        .arg("project-graph")
        .arg("foo")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn single_project_no_dependencies() {
    let assert = create_moon_command("projects")
        .arg("project-graph")
        .arg("baz")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}
