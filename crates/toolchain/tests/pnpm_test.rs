use moon_config::{NodePackageManager, PnpmConfig, WorkspaceConfig};
use moon_lang_node::node;
use moon_toolchain::{Executable, Installable, Toolchain};
use predicates::prelude::*;
use std::env;
use std::path::PathBuf;

async fn create_pnpm_tool() -> (Toolchain, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    config.node.version = String::from("1.0.0");
    config.node.package_manager = NodePackageManager::Pnpm;
    config.node.pnpm = Some(PnpmConfig {
        version: String::from("6.0.0"),
    });

    let toolchain = Toolchain::create_from_dir(base_dir.path(), &env::temp_dir(), &config)
        .await
        .unwrap();

    (toolchain, base_dir)
}

#[tokio::test]
async fn generates_paths() {
    let (toolchain, temp_dir) = create_pnpm_tool().await;
    let pnpm = toolchain.get_node().get_pnpm().unwrap();

    assert!(predicates::str::ends_with(
        PathBuf::from(".moon")
            .join("tools")
            .join("node")
            .join("1.0.0")
            .to_str()
            .unwrap()
    )
    .eval(pnpm.get_install_dir().unwrap().to_str().unwrap()));

    let bin_path = PathBuf::from(".moon")
        .join("tools")
        .join("node")
        .join("1.0.0")
        .join(node::get_bin_name_suffix("pnpm", "cmd", false));

    assert!(predicates::str::ends_with(bin_path.to_str().unwrap())
        .eval(pnpm.get_bin_path().to_str().unwrap()));

    temp_dir.close().unwrap();
}
