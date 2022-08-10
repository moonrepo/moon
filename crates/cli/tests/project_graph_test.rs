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

mod aliases {
    use super::*;

    #[test]
    fn uses_ids_in_graph() {
        let fixture = create_sandbox("project-graph/aliases");

        let assert = create_moon_command(fixture.path())
            .arg("project-graph")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn can_focus_using_an_alias() {
        let fixture = create_sandbox("project-graph/aliases");

        let assert = create_moon_command(fixture.path())
            .arg("project-graph")
            .arg("@scope/pkg-foo")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn resolves_aliases_in_depends_on() {
        let fixture = create_sandbox("project-graph/aliases");

        let assert = create_moon_command(fixture.path())
            .arg("project-graph")
            .arg("noLang")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}
