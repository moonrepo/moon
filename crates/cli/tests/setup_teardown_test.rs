use moon_utils::is_ci;
use moon_utils::path::get_home_dir;
use moon_utils::test::{create_fixtures_sandbox, create_moon_command_in};

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

    let fixture = create_fixtures_sandbox("cases");

    let setup = create_moon_command_in(fixture.path())
        .arg("setup")
        .env("MOON_NODE_VERSION", node_version)
        .assert();

    setup.success().code(0);

    assert!(node_dir.exists());

    let teardown = create_moon_command_in(fixture.path())
        .arg("teardown")
        .env("MOON_NODE_VERSION", node_version)
        .assert();

    teardown.success().code(0);

    assert!(!node_dir.exists());
}
