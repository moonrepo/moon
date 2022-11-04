use moon_config::{NodeConfig, NpmConfig, WorkspaceConfig};
use moon_toolchain::tools::node::NodeTool;
use moon_toolchain::{Executable, Installable, Toolchain};
use predicates::prelude::*;
use std::path::PathBuf;

async fn create_npm_tool() -> (NodeTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let config = WorkspaceConfig {
        node: Some(NodeConfig {
            version: String::from("1.0.0"),
            npm: NpmConfig {
                version: String::from("6.0.0"),
            },
            ..NodeConfig::default()
        }),
        ..WorkspaceConfig::default()
    };

    let toolchain = Toolchain::load_from(base_dir.path(), &config)
        .await
        .unwrap();

    (
        NodeTool::new(&toolchain.get_paths(), config.node.as_ref().unwrap()).unwrap(),
        base_dir,
    )
}

#[tokio::test]
async fn generates_paths() {
    let (node, temp_dir) = create_npm_tool().await;
    let npm = node.get_npm().unwrap();

    assert!(predicates::str::ends_with(
        PathBuf::from(".moon")
            .join("tools")
            .join("npm")
            .join("6.0.0")
            .to_str()
            .unwrap()
    )
    .eval(npm.get_install_dir().unwrap().to_str().unwrap()));

    let bin_path = PathBuf::from(".moon")
        .join("tools")
        .join("npm")
        .join("6.0.0")
        .join("bin")
        .join("npm-cli.js");

    assert!(predicates::str::ends_with(bin_path.to_str().unwrap())
        .eval(npm.get_bin_path().to_str().unwrap()));

    temp_dir.close().unwrap();
}
