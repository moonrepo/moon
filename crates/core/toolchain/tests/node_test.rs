use moon_config::{NodeConfig, WorkspaceConfig};
use moon_node_lang::node;
use moon_toolchain::tools::node::NodeTool;
use moon_toolchain::{Downloadable, Executable, Installable, Toolchain};

async fn create_node_tool() -> (NodeTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let config = WorkspaceConfig {
        node: Some(NodeConfig {
            version: String::from("1.0.0"),
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

fn get_download_file() -> String {
    node::get_download_file("1.0.0").unwrap()
}

fn create_shasums(hash: &str) -> String {
    format!("{hash}  node-v1.0.0-darwin-arm64.tar.gz\n{hash}  node-v1.0.0-darwin-x64.tar.gz\n{hash}  node-v1.0.0-linux-x64.tar.gz\n{hash}  node-v1.0.0-win-x64.zip\n", hash = hash)
}

#[tokio::test]
async fn generates_paths() {
    let (node, temp_dir) = create_node_tool().await;

    assert_eq!(
        node.get_install_dir().unwrap(),
        &temp_dir
            .join(".moon")
            .join("tools")
            .join("node")
            .join("1.0.0")
    );

    assert_eq!(
        node.get_bin_path(),
        &temp_dir
            .join(".moon")
            .join("tools")
            .join("node")
            .join("1.0.0")
            .join(node::get_bin_name_suffix("node", "exe", false))
    );

    temp_dir.close().unwrap();
}

mod download {
    use super::*;
    use mockito::mock;

    #[tokio::test]
    async fn is_downloaded_checks() {
        let (node, temp_dir) = create_node_tool().await;

        assert!(!node.is_downloaded().await.unwrap());

        let dl_path = node.get_download_path().unwrap();

        std::fs::create_dir_all(dl_path.parent().unwrap()).unwrap();
        std::fs::write(dl_path, "").unwrap();

        assert!(node.is_downloaded().await.unwrap());

        std::fs::remove_file(dl_path).unwrap();

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn downloads_to_temp_dir() {
        let (node, temp_dir) = create_node_tool().await;

        assert!(!node.get_download_path().unwrap().exists());

        let archive = mock(
            "GET",
            format!("/dist/v1.0.0/{}", get_download_file()).as_str(),
        )
        .with_body("binary")
        .create();

        let shasums = mock("GET", "/dist/v1.0.0/SHASUMS256.txt")
            .with_body(create_shasums(
                "9a3a45d01531a20e89ac6ae10b0b0beb0492acd7216a368aa062d1a5fecaf9cd",
            ))
            .create();

        node.download(&(), Some(&mockito::server_url()))
            .await
            .unwrap();

        archive.assert();
        shasums.assert();

        assert!(node.get_download_path().unwrap().exists());

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "InvalidShasum")]
    async fn fails_on_invalid_shasum() {
        let (node, temp_dir) = create_node_tool().await;

        let archive = mock(
            "GET",
            format!("/dist/v1.0.0/{}", get_download_file()).as_str(),
        )
        .with_body("binary")
        .create();

        let shasums = mock("GET", "/dist/v1.0.0/SHASUMS256.txt")
            .with_body(create_shasums("fakehash"))
            .create();

        node.download(&(), Some(&mockito::server_url()))
            .await
            .unwrap();

        archive.assert();
        shasums.assert();

        assert!(node.get_download_path().unwrap().exists());

        temp_dir.close().unwrap();
    }
}
