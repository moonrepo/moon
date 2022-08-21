use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, create_sandbox, get_assert_output};

#[test]
fn all_by_default() {
    let fixture = create_sandbox("cases");

    let assert = create_moon_command(fixture.path())
        .arg("dep-graph")
        .assert();
    let dot = get_assert_output(&assert);

    // Snapshot is not deterministic
    assert_eq!(dot.split('\n').count(), 310);
}

#[test]
fn focused_by_target() {
    let fixture = create_sandbox("cases");

    let assert = create_moon_command(fixture.path())
        .arg("dep-graph")
        .arg("node:standard")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn includes_dependencies_when_focused() {
    let fixture = create_sandbox("cases");

    let assert = create_moon_command(fixture.path())
        .arg("dep-graph")
        .arg("dependsOn:standard")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn includes_dependents_when_focused() {
    let fixture = create_sandbox("cases");

    let assert = create_moon_command(fixture.path())
        .arg("dep-graph")
        .arg("depsC:standard")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

mod aliases {
    use super::*;

    #[test]
    fn can_focus_using_an_alias() {
        let fixture = create_sandbox("project-graph/aliases");

        let assert = create_moon_command(fixture.path())
            .arg("dep-graph")
            .arg("@scope/pkg-foo:test")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn resolves_aliases_in_task_deps() {
        let fixture = create_sandbox("project-graph/aliases");

        let assert = create_moon_command(fixture.path())
            .arg("dep-graph")
            .arg("node:aliasDeps")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}
