use moon_config::{PackageManager, PnpmConfig, WorkspaceConfig};
use moon_toolchain::tools::pnpm::PnpmTool;
use moon_toolchain::{Tool, Toolchain};
use predicates::prelude::*;
use std::env;
use std::path::PathBuf;

async fn create_pnpm_tool() -> (PnpmTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    config.node.version = String::from("1.0.0");
    config.node.package_manager = PackageManager::Pnpm;
    config.node.pnpm = Some(PnpmConfig {
        version: String::from("6.0.0"),
    });

    let toolchain = Toolchain::create_from_dir(&config, base_dir.path(), &env::temp_dir())
        .await
        .unwrap();

    (toolchain.get_pnpm().unwrap().to_owned(), base_dir)
}

#[tokio::test]
async fn generates_paths() {
    let (pnpm, temp_dir) = create_pnpm_tool().await;

    assert!(predicates::str::ends_with(
        PathBuf::from(".moon")
            .join("tools")
            .join("node")
            .join("1.0.0")
            .to_str()
            .unwrap()
    )
    .eval(pnpm.get_install_dir().to_str().unwrap()));

    let mut bin_path = PathBuf::from(".moon")
        .join("tools")
        .join("node")
        .join("1.0.0");

    if env::consts::OS == "windows" {
        bin_path = bin_path.join("pnpm");
    } else {
        bin_path = bin_path.join("bin").join("pnpm");
    }

    assert!(predicates::str::ends_with(bin_path.to_str().unwrap())
        .eval(pnpm.get_bin_path().to_str().unwrap()));

    temp_dir.close().unwrap();
}

mod install {
    // TODO, how to test subprocesses?
}
