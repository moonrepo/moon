use insta::assert_snapshot;
use moon_utils::test::{
    create_moon_command, create_sandbox, get_assert_output, get_assert_stderr_output_clean,
};

#[test]
fn unknown_project() {
    let fixture = create_sandbox("projects");

    let assert = create_moon_command(fixture.path())
        .arg("project")
        .arg("unknown")
        .assert();

    assert_snapshot!(get_assert_stderr_output_clean(&assert));

    assert.failure().code(1);
}

#[test]
fn empty_config() {
    let fixture = create_sandbox("projects");

    let assert = create_moon_command(fixture.path())
        .arg("project")
        .arg("emptyConfig")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn no_config() {
    let fixture = create_sandbox("projects");

    let assert = create_moon_command(fixture.path())
        .arg("project")
        .arg("noConfig")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn basic_config() {
    // with dependsOn and fileGroups
    let fixture = create_sandbox("projects");

    let assert = create_moon_command(fixture.path())
        .arg("project")
        .arg("basic")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn advanced_config() {
    // with project metadata
    let fixture = create_sandbox("projects");

    let assert = create_moon_command(fixture.path())
        .arg("project")
        .arg("advanced")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn depends_on_paths() {
    // shows dependsOn paths when they exist
    let fixture = create_sandbox("projects");

    let assert = create_moon_command(fixture.path())
        .arg("project")
        .arg("foo")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn with_tasks() {
    let fixture = create_sandbox("projects");

    let assert = create_moon_command(fixture.path())
        .arg("project")
        .arg("tasks")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn root_level() {
    let fixture = create_sandbox("cases");

    let assert = create_moon_command(fixture.path())
        .arg("project")
        .arg("root")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}
