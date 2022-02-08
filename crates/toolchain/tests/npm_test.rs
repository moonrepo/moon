use moon_config::{NpmConfig, WorkspaceConfig};
use moon_toolchain::tools::npm::NpmTool;
use moon_toolchain::{Tool, Toolchain};
use predicates::prelude::*;
use std::env;

async fn create_npm_tool() -> (NpmTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    if let Some(ref mut node) = config.node {
        node.version = String::from("1.0.0");
        node.npm = Some(NpmConfig {
            version: String::from("6.0.0"),
        });
    }

    let toolchain = Toolchain::create_from_dir(&config, base_dir.path(), &env::temp_dir())
        .await
        .unwrap();

    (toolchain.get_npm().to_owned(), base_dir)
}

#[tokio::test]
async fn generates_paths() {
    let (npm, temp_dir) = create_npm_tool().await;

    assert!(predicates::str::ends_with(".moon/tools/node/1.0.0")
        .eval(npm.get_install_dir().to_str().unwrap()));

    assert!(predicates::str::ends_with(".moon/tools/node/1.0.0/bin/npm")
        .eval(npm.get_bin_path().to_str().unwrap()));

    temp_dir.close().unwrap();
}

mod install {
    // TODO, how to test subprocesses?
}
