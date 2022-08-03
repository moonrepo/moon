use moon_config::{NodePackageManager, WorkspaceConfig, YarnConfig};
use moon_lang_node::node;
use moon_toolchain::{Executable, Installable, Toolchain};
use predicates::prelude::*;
use std::env;
use std::path::PathBuf;

async fn create_yarn_tool() -> (Toolchain, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    config.node.version = String::from("1.0.0");
    config.node.package_manager = NodePackageManager::Yarn;
    config.node.yarn = Some(YarnConfig {
        version: String::from("6.0.0"),
    });

    let toolchain = Toolchain::create_from_dir(base_dir.path(), &env::temp_dir(), &config)
        .await
        .unwrap();

    (toolchain, base_dir)
}

#[tokio::test]
async fn generates_paths() {
    let (toolchain, temp_dir) = create_yarn_tool().await;
    let yarn = toolchain.get_node().get_yarn().unwrap();

    assert!(predicates::str::ends_with(
        PathBuf::from(".moon")
            .join("tools")
            .join("node")
            .join("1.0.0")
            .to_str()
            .unwrap()
    )
    .eval(yarn.get_install_dir().unwrap().to_str().unwrap()));

    let bin_path = PathBuf::from(".moon")
        .join("tools")
        .join("node")
        .join("1.0.0")
        .join(node::get_bin_name_suffix("yarn", "cmd", false));

    assert!(predicates::str::ends_with(bin_path.to_str().unwrap())
        .eval(yarn.get_bin_path().to_str().unwrap()));

    temp_dir.close().unwrap();
}
