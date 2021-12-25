use moon_config::{PackageManager, WorkspaceConfig, YarnConfig};
use moon_toolchain::tools::yarn::YarnTool;
use moon_toolchain::{Tool, Toolchain};
use predicates::prelude::*;
use std::env;
use std::path::PathBuf;

async fn create_yarn_tool() -> (YarnTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    if let Some(ref mut node) = config.node {
        node.version = String::from("1.0.0");
        node.package_manager = Some(PackageManager::Yarn);
        node.yarn = Some(YarnConfig {
            version: String::from("6.0.0"),
        });
    }

    let toolchain = Toolchain::create_from_dir(&config, base_dir.path(), &env::temp_dir())
        .await
        .unwrap();

    (toolchain.get_yarn().unwrap().to_owned(), base_dir)
}

#[tokio::test]
async fn generates_paths() {
    let (yarn, temp_dir) = create_yarn_tool().await;

    assert!(predicates::str::ends_with(
        PathBuf::from(".moon")
            .join("tools")
            .join("node")
            .join("1.0.0")
            .to_str()
            .unwrap()
    )
    .eval(yarn.get_install_dir().to_str().unwrap()));

    let mut bin_path = PathBuf::from(".moon")
        .join("tools")
        .join("node")
        .join("1.0.0");

    if env::consts::OS == "windows" {
        bin_path = bin_path.join("yarn");
    } else {
        bin_path = bin_path.join("bin").join("yarn");
    }

    assert!(predicates::str::ends_with(bin_path.to_str().unwrap())
        .eval(yarn.get_bin_path().to_str().unwrap()));

    temp_dir.close().unwrap();
}

mod install {
    // TODO, how to test subprocesses?
}
