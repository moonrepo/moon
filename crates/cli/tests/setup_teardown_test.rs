use moon_test_utils::{create_sandbox_with_config, get_cases_fixture_configs};
use moon_utils::is_ci;
use moon_utils::path::get_home_dir;

#[test]
fn sets_up_and_tears_down() {
    // This is heavy so avoid in local tests for now
    if !is_ci() {
        return;
    }

    // We use a different Node.js version as to not conflict with other tests!
    let node_version = "17.1.0";
    let home_dir = get_home_dir().unwrap();
    let moon_dir = home_dir.join(".moon");
    let node_dir = moon_dir.join("tools/node").join(node_version);

    assert!(!node_dir.exists());

    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let setup = sandbox.run_moon(|cmd| {
        cmd.arg("setup").env("MOON_NODE_VERSION", node_version);
    });

    setup.success().code(0);

    assert!(node_dir.exists());

    let teardown = sandbox.run_moon(|cmd| {
        cmd.arg("teardown").env("MOON_NODE_VERSION", node_version);
    });

    teardown.success().code(0);

    assert!(!node_dir.exists());
}
