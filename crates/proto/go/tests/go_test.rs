use proto_core::{
    Detector, Downloadable, Executable, Installable, Proto, Resolvable, Shimable, Tool, Verifiable,
};
use proto_go::GoLanguage;

fn create_tool() -> (GoLanguage, assert_fs::TempDir) {
    let fixture = assert_fs::TempDir::new().unwrap();
    let mut tool = GoLanguage::new(&Proto::from(fixture.path()));
    (tool, fixture)
}

mod detector {
    use super::*;
    use assert_fs::prelude::{FileWriteStr, PathChild};

    #[tokio::test]
    async fn doesnt_match_if_no_files() {
        let (tool, fixture) = create_tool();

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn detects_gomod() {
        let (tool, fixture) = create_tool();

        fixture.child("go.mod").write_str("go 1.19").unwrap();

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            Some("1.19".into())
        );
    }
    //
    #[tokio::test]
    async fn detects_nodenv() {
        let (tool, fixture) = create_tool();

        fixture.child("go.work").write_str("go 1.19").unwrap();
        fixture.child("go.mod").write_str("go 1.18").unwrap();

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            Some("1.19".into())
        );
    }
}

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
    async fn resolve_alias_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = GoLanguage::new(&Proto::from(fixture.path()));

        assert_eq!(tool.resolve_version("1.11").await.unwrap(), "1.11.13");
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
