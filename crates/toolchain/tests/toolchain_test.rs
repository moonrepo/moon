use moon_config::WorkspaceConfig;
use moon_toolchain::Toolchain;
use predicates::prelude::*;
use std::env;
use std::path::{Path, PathBuf};

async fn create_toolchain(base_dir: &Path) -> Toolchain {
    let mut config = WorkspaceConfig::default();

    config.node.version = String::from("1.0.0");

    Toolchain::create_from_dir(base_dir, &env::temp_dir(), &config)
        .await
        .unwrap()
}

#[tokio::test]
async fn generates_paths() {
    let base_dir = assert_fs::TempDir::new().unwrap();
    let toolchain = create_toolchain(&base_dir).await;

    assert!(predicates::str::ends_with(".moon").eval(toolchain.dir.to_str().unwrap()));
    assert!(
        predicates::str::ends_with(PathBuf::from(".moon").join("temp").to_str().unwrap())
            .eval(toolchain.temp_dir.to_str().unwrap())
    );
    assert!(
        predicates::str::ends_with(PathBuf::from(".moon").join("tools").to_str().unwrap())
            .eval(toolchain.tools_dir.to_str().unwrap())
    );

    base_dir.close().unwrap();
}

#[tokio::test]
async fn creates_dirs() {
    let base_dir = assert_fs::TempDir::new().unwrap();
    let home_dir = base_dir.join(".moon");
    let temp_dir = base_dir.join(".moon/temp");
    let tools_dir = base_dir.join(".moon/tools");

    assert!(!home_dir.exists());
    assert!(!temp_dir.exists());
    assert!(!tools_dir.exists());

    create_toolchain(&base_dir).await;

    assert!(home_dir.exists());
    assert!(temp_dir.exists());
    assert!(tools_dir.exists());

    base_dir.close().unwrap();
}
