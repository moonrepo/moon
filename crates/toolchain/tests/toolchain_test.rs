use moon_config::{NodeConfig, WorkspaceConfig};
use moon_toolchain::Toolchain;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

async fn create_toolchain(base_dir: &Path) -> Toolchain {
    let config = WorkspaceConfig {
        node: Some(NodeConfig {
            version: String::from("1.0.0"),
            ..NodeConfig::default()
        }),
        ..WorkspaceConfig::default()
    };

    Toolchain::create_from(base_dir, &config).await.unwrap()
}

#[tokio::test]
async fn generates_paths() {
    let base_dir = assert_fs::TempDir::new().unwrap();
    let toolchain = create_toolchain(&base_dir).await;
    let paths = toolchain.get_paths();

    assert!(predicates::str::ends_with(".moon").eval(toolchain.dir.to_str().unwrap()));
    assert!(
        predicates::str::ends_with(PathBuf::from(".moon").join("temp").to_str().unwrap())
            .eval(paths.temp.to_str().unwrap())
    );
    assert!(
        predicates::str::ends_with(PathBuf::from(".moon").join("tools").to_str().unwrap())
            .eval(paths.tools.to_str().unwrap())
    );

    base_dir.close().unwrap();
}

#[tokio::test]
async fn creates_dirs() {
    let base_dir = assert_fs::TempDir::new().unwrap();
    let home_dir = base_dir.join(".moon");
    // let temp_dir = base_dir.join(".moon/temp");
    // let tools_dir = base_dir.join(".moon/tools");

    assert!(!home_dir.exists());
    // assert!(!temp_dir.exists());
    // assert!(!tools_dir.exists());

    create_toolchain(&base_dir).await;

    assert!(home_dir.exists());
    // assert!(temp_dir.exists());
    // assert!(tools_dir.exists());

    base_dir.close().unwrap();
}
