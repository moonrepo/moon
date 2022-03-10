use moon_config::WorkspaceConfig;
use moon_toolchain::tools::npm::NpmTool;
use moon_toolchain::{Tool, Toolchain};
use predicates::prelude::*;
use std::env;
use std::path::PathBuf;

async fn create_npm_tool() -> (NpmTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    config.node.version = String::from("1.0.0");
    config.node.npm.version = String::from("6.0.0");

    let toolchain = Toolchain::create_from_dir(&config, base_dir.path(), &env::temp_dir())
        .await
        .unwrap();

    (toolchain.get_npm().to_owned(), base_dir)
}

#[tokio::test]
async fn generates_paths() {
    let (npm, temp_dir) = create_npm_tool().await;

    assert!(predicates::str::ends_with(
        PathBuf::from(".moon")
            .join("tools")
            .join("node")
            .join("1.0.0")
            .to_str()
            .unwrap()
    )
    .eval(npm.get_install_dir().to_str().unwrap()));

    let mut bin_path = PathBuf::from(".moon")
        .join("tools")
        .join("node")
        .join("1.0.0");

    if env::consts::OS == "windows" {
        bin_path = bin_path.join("npm");
    } else {
        bin_path = bin_path.join("bin").join("npm");
    }

    assert!(predicates::str::ends_with(bin_path.to_str().unwrap())
        .eval(npm.get_bin_path().to_str().unwrap()));

    temp_dir.close().unwrap();
}

mod install {
    // TODO, how to test subprocesses?
}
