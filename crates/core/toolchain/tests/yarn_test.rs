use moon_config::{NodeConfig, NodePackageManager, WorkspaceConfig, YarnConfig};
use moon_toolchain::tools::node::NodeTool;
use moon_toolchain::{Executable, Installable, Toolchain};

async fn create_yarn_tool() -> (NodeTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let config = WorkspaceConfig {
        node: Some(NodeConfig {
            version: String::from("1.0.0"),
            package_manager: NodePackageManager::Yarn,
            yarn: Some(YarnConfig {
                plugins: None,
                version: String::from("1.0.0"),
            }),
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
    let (node, temp_dir) = create_yarn_tool().await;
    let yarn = node.get_yarn().unwrap();

    assert_eq!(
        yarn.get_install_dir().unwrap(),
        &temp_dir
            .join(".moon")
            .join("tools")
            .join("yarn")
            .join("1.0.0")
    );

    assert_eq!(
        yarn.get_bin_path(),
        &temp_dir
            .join(".moon")
            .join("tools")
            .join("yarn")
            .join("1.0.0")
            .join("bin")
            .join("yarn.js")
    );

    temp_dir.close().unwrap();
}
