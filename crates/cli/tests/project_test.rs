use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, get_assert_output, get_assert_stderr_output};

#[test]
fn unknown_project() {
    let assert = create_moon_command("projects")
        .arg("project")
        .arg("unknown")
        .assert();

    assert_snapshot!(get_assert_stderr_output(&assert));

    assert.failure().code(1);
}

#[test]
fn empty_config() {
    let assert = create_moon_command("projects")
        .arg("project")
        .arg("emptyConfig")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn no_config() {
    let assert = create_moon_command("projects")
        .arg("project")
        .arg("noConfig")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn basic_config() {
    // with dependsOn and fileGroups
    let assert = create_moon_command("projects")
        .arg("project")
        .arg("basic")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn advanced_config() {
    // with project metadata
    let assert = create_moon_command("projects")
        .arg("project")
        .arg("advanced")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn depends_on_paths() {
    // shows dependsOn paths when they exist
    let assert = create_moon_command("projects")
        .arg("project")
        .arg("foo")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn with_tasks() {
    let assert = create_moon_command("projects")
        .arg("project")
        .arg("tasks")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}
