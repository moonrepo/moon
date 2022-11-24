use std::path::Path;

use probe_core::{Probe, Resolvable};
use probe_node::NodeLanguage;

fn create_probe(dir: &Path) -> Probe {
    Probe {
        temp_dir: dir.join("temp"),
        tools_dir: dir.join("tools"),
    }
}

mod resolver {
    use super::*;

    #[tokio::test]
    async fn updates_struct_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()));

        assert_ne!(tool.resolve_version("node", None).await.unwrap(), "node");
        assert_ne!(tool.get_resolved_version(), "node");
    }

    #[tokio::test]
    async fn resolve_latest() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()));

        assert_ne!(
            tool.resolve_version("latest", None).await.unwrap(),
            "latest"
        );
    }

    #[tokio::test]
    async fn resolve_stable() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()));

        assert_ne!(
            tool.resolve_version("stable", None).await.unwrap(),
            "stable"
        );
    }

    #[tokio::test]
    async fn resolve_lts_wild() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()));

        assert_ne!(tool.resolve_version("lts-*", None).await.unwrap(), "lts-*");
    }

    #[tokio::test]
    async fn resolve_lts_dash() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()));

        assert_ne!(
            tool.resolve_version("lts-gallium", None).await.unwrap(),
            "lts-gallium"
        );
    }

    #[tokio::test]
    async fn resolve_lts_slash() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()));

        assert_ne!(
            tool.resolve_version("lts/gallium", None).await.unwrap(),
            "lts/gallium"
        );
    }

    #[tokio::test]
    async fn resolve_lts() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()));

        assert_ne!(
            tool.resolve_version("Gallium", None).await.unwrap(),
            "Gallium"
        );
    }

    #[tokio::test]
    async fn resolve_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()));

        assert_eq!(
            tool.resolve_version("18.0.0", None).await.unwrap(),
            "18.0.0"
        );
    }

    #[tokio::test]
    async fn resolve_version_with_prefix() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeLanguage::new(&create_probe(fixture.path()));

        assert_eq!(
            tool.resolve_version("v18.0.0", None).await.unwrap(),
            "18.0.0"
        );
    }
}
