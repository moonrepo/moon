use moon_config::{NodeConfig, NpmConfig};
use moon_lang_node::node;
use moon_toolchain::tools::node::NodeTool;
use moon_toolchain::{Executable, Installable, Toolchain};
use predicates::prelude::*;
use std::env;
use std::path::PathBuf;

async fn create_npm_tool() -> (NodeTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();
    let toolchain = Toolchain::create_from_dir(base_dir.path(), &env::temp_dir())
        .await
        .unwrap();

    (
        NodeTool::new(
            toolchain.get_paths(),
            &NodeConfig {
                version: String::from("1.0.0"),
                npm: NpmConfig {
                    version: String::from("6.0.0"),
                },
                ..NodeConfig::default()
            },
        )
        .unwrap(),
        base_dir,
    )
}

#[tokio::test]
async fn generates_paths() {
    let (node, temp_dir) = create_npm_tool().await;
    let npm = node.get_npm();

    assert!(predicates::str::ends_with(
        PathBuf::from(".moon")
            .join("tools")
            .join("node")
            .join("1.0.0")
            .to_str()
            .unwrap()
    )
    .eval(npm.get_install_dir().unwrap().to_str().unwrap()));

    let bin_path = PathBuf::from(".moon")
        .join("tools")
        .join("node")
        .join("1.0.0")
        .join(node::get_bin_name_suffix("npm", "cmd", false));

    assert!(predicates::str::ends_with(bin_path.to_str().unwrap())
        .eval(npm.get_bin_path().to_str().unwrap()));

    temp_dir.close().unwrap();
}
