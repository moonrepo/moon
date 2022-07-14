use insta::assert_snapshot;
use moon_utils::test::{
    create_fixtures_skeleton_sandbox, create_moon_command_in, get_assert_output,
};

#[test]
fn syncs_all_projects() {
    let fixture = create_fixtures_skeleton_sandbox("project-graph/dependencies");

    let assert = create_moon_command_in(fixture.path()).arg("sync").assert();

    assert_snapshot!(get_assert_output(&assert));

    assert.success();
}
