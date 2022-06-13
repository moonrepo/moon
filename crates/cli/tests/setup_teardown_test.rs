use moon_cli::commands::setup::setup;
use moon_cli::commands::teardown::teardown;
use moon_utils::is_ci;
use moon_utils::path::get_home_dir;
use moon_utils::test::create_fixtures_sandbox;
use std::env;

#[tokio::test]
async fn sets_up_and_tears_down() {
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

    env::set_var("MOON_NODE_VERSION", node_version);
    env::set_current_dir(fixture.path()).unwrap();

    setup().await.unwrap();

    assert!(node_dir.exists());

    teardown().await.unwrap();

    env::remove_var("MOON_NODE_VERSION");

    assert!(!node_dir.exists());
}
