use proto_core::{Downloadable, Executable, Installable, Proto, Resolvable, Tool, Verifiable};
use proto_node::{NodeDependencyManager, NodeDependencyManagerType};
use std::path::Path;

fn create_proto(dir: &Path) -> Proto {
    Proto {
        temp_dir: dir.join("temp"),
        tools_dir: dir.join("tools"),
    }
}

#[tokio::test]
async fn downloads_verifies_installs_npm() {
    let fixture = assert_fs::TempDir::new().unwrap();
    let proto = create_proto(fixture.path());
    let mut tool =
        NodeDependencyManager::new(&proto, NodeDependencyManagerType::Npm, Some("9.0.0"));

    tool.setup("9.0.0").await.unwrap();

    assert!(!tool.get_download_path().unwrap().exists());
    assert!(!tool.get_checksum_path().unwrap().exists());
    assert!(tool.get_install_dir().unwrap().exists());

    assert_eq!(
        tool.get_bin_path().unwrap(),
        &proto.tools_dir.join("npm/9.0.0/bin/npm-cli.js")
    );
}

#[tokio::test]
async fn downloads_verifies_installs_pnpm() {
    let fixture = assert_fs::TempDir::new().unwrap();
    let proto = create_proto(fixture.path());
    let mut tool =
        NodeDependencyManager::new(&proto, NodeDependencyManagerType::Pnpm, Some("7.0.0"));

    tool.setup("7.0.0").await.unwrap();

    assert!(!tool.get_download_path().unwrap().exists());
    assert!(!tool.get_checksum_path().unwrap().exists());
    assert!(tool.get_install_dir().unwrap().exists());

    assert_eq!(
        tool.get_bin_path().unwrap(),
        &proto.tools_dir.join("pnpm/7.0.0/bin/pnpm.cjs")
    );
}

#[tokio::test]
async fn downloads_verifies_installs_yarn_classic() {
    let fixture = assert_fs::TempDir::new().unwrap();
    let proto = create_proto(fixture.path());
    let mut tool =
        NodeDependencyManager::new(&proto, NodeDependencyManagerType::Yarn, Some("1.22.0"));

    tool.setup("1.22.0").await.unwrap();

    assert!(!tool.get_download_path().unwrap().exists());
    assert!(!tool.get_checksum_path().unwrap().exists());
    assert!(tool.get_install_dir().unwrap().exists());

    assert_eq!(
        tool.get_bin_path().unwrap(),
        &proto.tools_dir.join("yarn/1.22.0/bin/yarn.js")
    );
}

#[tokio::test]
async fn downloads_verifies_installs_yarn_berry() {
    let fixture = assert_fs::TempDir::new().unwrap();
    let proto = create_proto(fixture.path());
    let mut tool =
        NodeDependencyManager::new(&proto, NodeDependencyManagerType::Yarn, Some("3.0.0"));

    tool.setup("3.0.0").await.unwrap();

    assert!(!tool.get_download_path().unwrap().exists());
    assert!(!tool.get_checksum_path().unwrap().exists());
    assert!(tool.get_install_dir().unwrap().exists());

    assert_eq!(tool.get_resolved_version(), "1.22.19");
    assert_eq!(
        tool.get_bin_path().unwrap(),
        &proto.tools_dir.join("yarn/1.22.19/bin/yarn.js")
    );
}

mod downloader {
    use super::*;

    #[tokio::test]
    async fn sets_path_to_temp() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let proto = create_proto(fixture.path());
        let tool =
            NodeDependencyManager::new(&proto, NodeDependencyManagerType::Npm, Some("9.0.0"));

        assert_eq!(
            tool.get_download_path().unwrap(),
            proto.temp_dir.join("npm").join("9.0.0.tgz")
        );
    }

    #[tokio::test]
    async fn downloads_to_temp() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let tool = NodeDependencyManager::new(
            &create_proto(fixture.path()),
            NodeDependencyManagerType::Npm,
            Some("9.0.0"),
        );

        let to_file = tool.get_download_path().unwrap();

        assert!(!to_file.exists());

        tool.download(&to_file, None).await.unwrap();

        assert!(to_file.exists());
    }

    #[tokio::test]
    async fn doesnt_download_if_file_exists() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let tool = NodeDependencyManager::new(
            &create_proto(fixture.path()),
            NodeDependencyManagerType::Npm,
            Some("9.0.0"),
        );

        let to_file = tool.get_download_path().unwrap();

        assert!(tool.download(&to_file, None).await.unwrap());
        assert!(!tool.download(&to_file, None).await.unwrap());
    }
}

mod resolver {
    use super::*;

    #[tokio::test]
    async fn resolve_latest() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &create_proto(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        assert_ne!(
            tool.resolve_version("latest", None).await.unwrap(),
            "latest"
        );
        assert_ne!(tool.get_resolved_version(), "latest");
    }

    #[tokio::test]
    async fn resolve_custom_dist() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &create_proto(fixture.path()),
            NodeDependencyManagerType::Yarn,
            None,
        );

        assert_ne!(tool.resolve_version("berry", None).await.unwrap(), "berry");
    }

    #[tokio::test]
    async fn handles_npm() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &create_proto(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        assert_eq!(tool.resolve_version("9.0.0", None).await.unwrap(), "9.0.0");
    }

    #[tokio::test]
    async fn handles_pnpm() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &create_proto(fixture.path()),
            NodeDependencyManagerType::Pnpm,
            None,
        );

        assert_eq!(tool.resolve_version("7.0.0", None).await.unwrap(), "7.0.0");
    }

    #[tokio::test]
    async fn handles_yarn() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &create_proto(fixture.path()),
            NodeDependencyManagerType::Yarn,
            None,
        );

        assert_eq!(
            tool.resolve_version("1.22.0", None).await.unwrap(),
            "1.22.0"
        );
    }

    #[tokio::test]
    #[should_panic(expected = "VersionUnknownAlias(\"unknown\")")]
    async fn errors_invalid_alias() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &create_proto(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        tool.resolve_version("unknown", None).await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "VersionResolveFailed(\"99.99.99\")")]
    async fn errors_invalid_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &create_proto(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        tool.resolve_version("99.99.99", None).await.unwrap();
    }
}
