use monolith_config::WorkspaceConfig;
use monolith_toolchain::tools::node::NodeTool;
use monolith_toolchain::{Tool, Toolchain};
use predicates::prelude::*;
use std::env;

fn create_node_tool() -> (NodeTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    config.node.version = String::from("1.0.0");

    let toolchain = Toolchain::from(&config, base_dir.path(), &env::temp_dir()).unwrap();

    (toolchain.get_node().to_owned(), base_dir)
}

fn get_node_platform() -> &'static str {
    if env::consts::OS == "windows" {
        "win"
    } else if env::consts::OS == "macos" {
        "darwin"
    } else {
        "linux"
    }
}

#[test]
fn generates_paths() {
    let (node, temp_dir) = create_node_tool();

    assert!(predicates::str::ends_with(".monolith/tools/node/1.0.0")
        .eval(node.get_install_dir().to_str().unwrap()));

    assert!(
        predicates::str::ends_with(".monolith/tools/node/1.0.0/bin/node")
            .eval(node.get_bin_path().to_str().unwrap())
    );

    assert!(predicates::str::ends_with(format!(
        ".monolith/temp/node/node-v1.0.0-{}-x64.tar.gz",
        get_node_platform()
    ))
    .eval(node.get_download_path().unwrap().to_str().unwrap()));

    temp_dir.close().unwrap();
}

mod download {
    use super::*;
    use mockito::mock;

    #[test]
    fn is_downloaded_checks() {
        let (node, temp_dir) = create_node_tool();

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
        let (node, temp_dir) = create_node_tool();

        assert!(!node.get_download_path().unwrap().exists());

        let archive = mock(
            "GET",
            format!(
                "/dist/v1.0.0/node-v1.0.0-{}-x64.tar.gz",
                get_node_platform()
            )
            .as_str(),
        )
        .with_body("binary")
        .create();

        let shasums = mock("GET", "/dist/v1.0.0/SHASUMS256.txt")
            .with_body("9a3a45d01531a20e89ac6ae10b0b0beb0492acd7216a368aa062d1a5fecaf9cd  node-v1.0.0-darwin-x64.tar.gz\n9a3a45d01531a20e89ac6ae10b0b0beb0492acd7216a368aa062d1a5fecaf9cd  node-v1.0.0-linux-x64.tar.gz\n")
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
        let (node, temp_dir) = create_node_tool();

        let archive = mock(
            "GET",
            format!(
                "/dist/v1.0.0/node-v1.0.0-{}-x64.tar.gz",
                get_node_platform()
            )
            .as_str(),
        )
        .with_body("binary")
        .create();

        let shasums = mock("GET", "/dist/v1.0.0/SHASUMS256.txt")
            .with_body(
                "fakehash  node-v1.0.0-darwin-x64.tar.gz\nfakehash  node-v1.0.0-linux-x64.tar.gz\n",
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
