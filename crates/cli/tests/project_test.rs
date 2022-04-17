use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, get_assert_output, get_assert_stderr_output};

fn force_ansi_colors() {
    std::env::set_var("CLICOLOR_FORCE", "1");
}

#[test]
fn unknown_project() {
    force_ansi_colors();

    let assert = create_moon_command("projects")
        .arg("project")
        .arg("unknown")
        .assert();

    assert_snapshot!(get_assert_stderr_output(&assert));

    assert.failure().code(1);
}

#[test]
fn empty_config() {
    force_ansi_colors();

    let assert = create_moon_command("projects")
        .arg("project")
        .arg("emptyConfig")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn no_config() {
    force_ansi_colors();

    let assert = create_moon_command("projects")
        .arg("project")
        .arg("noConfig")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn basic_config() {
    force_ansi_colors();

    // with dependsOn and fileGroups
    let assert = create_moon_command("projects")
        .arg("project")
        .arg("basic")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn advanced_config() {
    force_ansi_colors();

    // with project metadata
    let assert = create_moon_command("projects")
        .arg("project")
        .arg("advanced")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn depends_on_paths() {
    force_ansi_colors();

    // shows dependsOn paths when they exist
    let assert = create_moon_command("projects")
        .arg("project")
        .arg("foo")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn with_tasks() {
    force_ansi_colors();

    let assert = create_moon_command("projects")
        .arg("project")
        .arg("tasks")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}
