use moon_test_utils::{
    create_sandbox_with_config, get_cases_fixture_configs, predicates::prelude::*,
};
use proto_core::VersionReq;

#[test]
fn fails_on_version_constraint() {
    let (mut workspace_config, _, _) = get_cases_fixture_configs();

    workspace_config.version_constraint = Some(VersionReq::parse(">=1000.0.0").unwrap());

    let sandbox = create_sandbox_with_config("cases", Some(workspace_config), None, None);

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("sync");
    });

    assert
        .failure()
        .stderr(predicate::str::contains(">=1000.0.0"));
}
