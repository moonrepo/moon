use moon_config::{NodeConfig, NpmConfig, WorkspaceConfig};
use moon_toolchain::tools::node::NodeTool;
use moon_toolchain::{Executable, Installable, Toolchain};

async fn create_npm_tool() -> (NodeTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let config = WorkspaceConfig {
        node: Some(NodeConfig {
            version: String::from("1.0.0"),
            npm: NpmConfig {
                version: String::from("6.0.0"),
            },
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
    let (node, temp_dir) = create_npm_tool().await;
    let npm = node.get_npm().unwrap();

    assert_eq!(
        npm.get_install_dir().unwrap(),
        &temp_dir
            .join(".moon")
            .join("tools")
            .join("npm")
            .join("6.0.0")
    );

    assert_eq!(
        npm.get_bin_path(),
        &temp_dir
            .join(".moon")
            .join("tools")
            .join("npm")
            .join("6.0.0")
            .join("bin")
            .join("npm-cli.js")
    );

    temp_dir.close().unwrap();
}
