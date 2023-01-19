use proto_core::{
    Detector, Downloadable, Executable, Installable, Proto, Resolvable, Shimable, Tool, Verifiable,
};
use proto_go::GoLanguage;

mod resolver {
    use super::*;

    #[tokio::test]
    async fn resolve_base_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = GoLanguage::new(&Proto::from(fixture.path()));

        assert_ne!(tool.resolve_version("1.19").await.unwrap(), "1.19");
        assert_ne!(tool.resolve_version("1.19").await.unwrap(), "1.19.0");
    }

    #[tokio::test]
    async fn resolve_specific_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = GoLanguage::new(&Proto::from(fixture.path()));

        assert_eq!(tool.resolve_version("1.9.2").await.unwrap(), "1.9.2");
    }

    #[tokio::test]
    async fn resolve_rc_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = GoLanguage::new(&Proto::from(fixture.path()));

        assert_eq!(tool.resolve_version("1.9rc2").await.unwrap(), "1.9rc2");
    }
}
