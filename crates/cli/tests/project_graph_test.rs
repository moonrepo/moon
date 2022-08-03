use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, create_sandbox, get_assert_output};

#[test]
fn no_projects() {
    let fixture = create_sandbox("base");

    let assert = create_moon_command(fixture.path())
        .arg("project-graph")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn many_projects() {
    let fixture = create_sandbox("projects");

    let assert = create_moon_command(fixture.path())
        .arg("project-graph")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn single_project_with_dependencies() {
    let fixture = create_sandbox("projects");

    let assert = create_moon_command(fixture.path())
        .arg("project-graph")
        .arg("foo")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn single_project_no_dependencies() {
    let fixture = create_sandbox("projects");

    let assert = create_moon_command(fixture.path())
        .arg("project-graph")
        .arg("baz")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}
