use moon_test_utils::{assert_snapshot, create_sandbox_with_config, get_cases_fixture_configs};

#[test]
fn fails_on_version_constraint() {
    let (mut workspace_config, _, _) = get_cases_fixture_configs();

    workspace_config.version_constraint = Some(">=1000.0.0".into());

    let sandbox = create_sandbox_with_config("cases", Some(&workspace_config), None, None);

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("sync");
    });

    assert_snapshot!(assert.output());

    assert.failure();
}
