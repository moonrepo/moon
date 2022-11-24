use std::path::Path;

use probe_core::{Downloadable, Probe, Resolvable, Verifiable};
use probe_node::NodeLanguage;

fn create_probe(dir: &Path) -> Probe {
    Probe {
        temp_dir: dir.join("temp"),
        tools_dir: dir.join("tools"),
    }
}

mod downloader {
    use super::*;
    use probe_node::download::get_archive_file;

    #[tokio::test]
    async fn sets_path_to_temp() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let probe = create_probe(fixture.path());
        let tool = NodeLanguage::new(&probe, Some("18.0.0"));

        assert_eq!(
            tool.get_download_path().unwrap(),
            probe
                .temp_dir
                .join("node")
                .join(get_archive_file("18.0.0").unwrap())
        );
    }

    #[tokio::test]
    async fn downloads_to_temp() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let tool = NodeLanguage::new(&create_probe(fixture.path()), Some("18.0.0"));

        let to_file = tool.get_download_path().unwrap();

        assert!(!to_file.exists());

        tool.download(&to_file, None).await.unwrap();

        assert!(to_file.exists());
    }

    #[tokio::test]
    async fn doesnt_download_if_file_exists() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let tool = NodeLanguage::new(&create_probe(fixture.path()), Some("18.0.0"));

        let to_file = tool.get_download_path().unwrap();

        assert!(tool.download(&to_file, None).await.unwrap());
        assert!(!tool.download(&to_file, None).await.unwrap());
    }
}

mod resolver {
    use super::*;

    #[tokio::test]
    async fn updates_struct_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()), None);

        assert_ne!(tool.resolve_version("node", None).await.unwrap(), "node");
        assert_ne!(tool.get_resolved_version(), "node");
    }

    #[tokio::test]
    async fn resolve_latest() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()), None);

        assert_ne!(
            tool.resolve_version("latest", None).await.unwrap(),
            "latest"
        );
    }

    #[tokio::test]
    async fn resolve_stable() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()), None);

        assert_ne!(
            tool.resolve_version("stable", None).await.unwrap(),
            "stable"
        );
    }

    #[tokio::test]
    async fn resolve_lts_wild() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()), None);

        assert_ne!(tool.resolve_version("lts-*", None).await.unwrap(), "lts-*");
    }

    #[tokio::test]
    async fn resolve_lts_dash() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()), None);

        assert_ne!(
            tool.resolve_version("lts-gallium", None).await.unwrap(),
            "lts-gallium"
        );
    }

    #[tokio::test]
    async fn resolve_lts_slash() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()), None);

        assert_ne!(
            tool.resolve_version("lts/gallium", None).await.unwrap(),
            "lts/gallium"
        );
    }

    #[tokio::test]
    async fn resolve_lts() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()), None);

        assert_ne!(
            tool.resolve_version("Gallium", None).await.unwrap(),
            "Gallium"
        );
    }

    #[tokio::test]
    async fn resolve_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()), None);

        assert_eq!(
            tool.resolve_version("18.0.0", None).await.unwrap(),
            "18.0.0"
        );
    }

    #[tokio::test]
    async fn resolve_version_with_prefix() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()), None);

        assert_eq!(
            tool.resolve_version("v18.0.0", None).await.unwrap(),
            "18.0.0"
        );
    }
}

mod verifier {
    use super::*;

    #[tokio::test]
    async fn sets_path_to_temp() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let probe = create_probe(fixture.path());
        let tool = NodeLanguage::new(&probe, Some("18.0.0"));

        assert_eq!(
            tool.get_checksum_path().unwrap(),
            probe.temp_dir.join("node").join("18.0.0-SHASUMS256.txt")
        );
    }

    #[tokio::test]
    async fn downloads_to_temp() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let tool = NodeLanguage::new(&create_probe(fixture.path()), Some("18.0.0"));
        let to_file = tool.get_checksum_path().unwrap();

        assert!(!to_file.exists());

        tool.download_checksum(&to_file, None).await.unwrap();

        assert!(to_file.exists());
    }

    #[tokio::test]
    async fn doesnt_download_if_file_exists() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let tool = NodeLanguage::new(&create_probe(fixture.path()), Some("18.0.0"));

        let to_file = tool.get_checksum_path().unwrap();

        assert!(tool.download_checksum(&to_file, None).await.unwrap());
        assert!(!tool.download_checksum(&to_file, None).await.unwrap());
    }
}
