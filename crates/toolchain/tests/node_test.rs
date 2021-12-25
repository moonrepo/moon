use moon_config::WorkspaceConfig;
use moon_toolchain::tools::node::NodeTool;
use moon_toolchain::{Tool, Toolchain};
use predicates::prelude::*;
use std::env;
use std::path::PathBuf;

async fn create_node_tool() -> (NodeTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    if let Some(ref mut node) = config.node {
        node.version = String::from("1.0.0");
    }

    let toolchain = Toolchain::create_from_dir(&config, base_dir.path(), &env::temp_dir())
        .await
        .unwrap();

    (toolchain.get_node().to_owned(), base_dir)
}

fn get_download_file() -> &'static str {
    if env::consts::OS == "windows" {
        "node-v1.0.0-win-x64.zip"
    } else if env::consts::OS == "macos" {
        "node-v1.0.0-darwin-x64.tar.gz"
    } else {
        "node-v1.0.0-linux-x64.tar.gz"
    }
}

#[tokio::test]
async fn generates_paths() {
    let (node, temp_dir) = create_node_tool().await;

    println!(
        "install = {}, {}",
        node.get_install_dir().to_str().unwrap(),
        PathBuf::from(".moon")
            .join("tools")
            .join("node")
            .join("1.0.0")
            .to_str()
            .unwrap()
    );

    // We have to use join a lot to test on windows
    assert!(predicates::str::ends_with(
        PathBuf::from(".moon")
            .join("tools")
            .join("node")
            .join("1.0.0")
            .to_str()
            .unwrap()
    )
    .eval(node.get_install_dir().to_str().unwrap()));

    let mut bin_path = PathBuf::from(".moon")
        .join("tools")
        .join("node")
        .join("1.0.0");

    if env::consts::OS == "windows" {
        bin_path = bin_path.join("node.exe");
    } else {
        bin_path = bin_path.join("bin").join("node");
    }

    assert!(predicates::str::ends_with(bin_path.to_str().unwrap())
        .eval(node.get_bin_path().to_str().unwrap()));

    assert!(predicates::str::ends_with(
        PathBuf::from(".moon")
            .join("temp")
            .join("node")
            .join(get_download_file())
            .to_str()
            .unwrap()
    )
    .eval(node.get_download_path().unwrap().to_str().unwrap()));

    temp_dir.close().unwrap();
}

mod download {
    use super::*;
    use mockito::mock;

    #[tokio::test]
    async fn is_downloaded_checks() {
        let (node, temp_dir) = create_node_tool().await;

        assert!(!node.is_downloaded());

        let dl_path = node.get_download_path().unwrap();

        std::fs::create_dir_all(dl_path.parent().unwrap()).unwrap();
        std::fs::write(dl_path, "").unwrap();

        assert!(node.is_downloaded());

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
            .with_body("9a3a45d01531a20e89ac6ae10b0b0beb0492acd7216a368aa062d1a5fecaf9cd  node-v1.0.0-darwin-x64.tar.gz\n9a3a45d01531a20e89ac6ae10b0b0beb0492acd7216a368aa062d1a5fecaf9cd  node-v1.0.0-linux-x64.tar.gz\n9a3a45d01531a20e89ac6ae10b0b0beb0492acd7216a368aa062d1a5fecaf9cd  node-v1.0.0-win-x64.zip\n")
            .create();

        node.download(Some(&mockito::server_url())).await.unwrap();

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
            .with_body(
                "fakehash  node-v1.0.0-darwin-x64.tar.gz\nfakehash  node-v1.0.0-linux-x64.tar.gz\nfakehash  node-v1.0.0-win-x64.zip\n",
            )
            .create();

        node.download(Some(&mockito::server_url())).await.unwrap();

        archive.assert();
        shasums.assert();

        assert!(node.get_download_path().unwrap().exists());

        temp_dir.close().unwrap();
    }
}

mod install {
    // TODO, how to test unzipping? and mocking subprocesses?
}
