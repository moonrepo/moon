use probe_core::{Downloadable, Probe, Resolvable};
use probe_node::{NodeDependencyManager, NodeDependencyManagerType};
use std::path::Path;

fn create_probe(dir: &Path) -> Probe {
    Probe {
        temp_dir: dir.join("temp"),
        tools_dir: dir.join("tools"),
    }
}

mod downloader {
    use super::*;

    #[tokio::test]
    async fn sets_path_to_temp() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let probe = create_probe(fixture.path());
        let tool =
            NodeDependencyManager::new(&probe, NodeDependencyManagerType::Npm, Some("9.0.0"));

        assert_eq!(
            tool.get_download_path().unwrap(),
            probe.temp_dir.join("npm").join("9.0.0.tgz")
        );
    }

    #[tokio::test]
    async fn downloads_to_temp() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let tool = NodeDependencyManager::new(
            &create_probe(fixture.path()),
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
            &create_probe(fixture.path()),
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
            &create_probe(fixture.path()),
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
            &create_probe(fixture.path()),
            NodeDependencyManagerType::Yarn,
            None,
        );

        assert_ne!(tool.resolve_version("berry", None).await.unwrap(), "berry");
    }

    #[tokio::test]
    async fn handles_npm() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &create_probe(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        assert_eq!(tool.resolve_version("9.0.0", None).await.unwrap(), "9.0.0");
    }

    #[tokio::test]
    async fn handles_pnpm() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &create_probe(fixture.path()),
            NodeDependencyManagerType::Pnpm,
            None,
        );

        assert_eq!(tool.resolve_version("7.0.0", None).await.unwrap(), "7.0.0");
    }

    #[tokio::test]
    async fn handles_yarn() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &create_probe(fixture.path()),
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
            &create_probe(fixture.path()),
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
            &create_probe(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        tool.resolve_version("99.99.99", None).await.unwrap();
    }
}
