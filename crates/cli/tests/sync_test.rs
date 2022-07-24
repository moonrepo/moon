use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, create_sandbox, get_assert_output};

#[test]
fn syncs_all_projects() {
    let fixture = create_sandbox("project-graph/dependencies");

    let assert = create_moon_command(fixture.path()).arg("sync").assert();

    assert_snapshot!(get_assert_output(&assert));

    assert.success();
}
