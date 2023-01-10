use proto_core::{
    Detector, Downloadable, Executable, Installable, Proto, Resolvable, Shimable, Tool,
};
use proto_node::{NodeDependencyManager, NodeDependencyManagerType};

#[tokio::test]
async fn downloads_verifies_installs_npm() {
    let fixture = assert_fs::TempDir::new().unwrap();
    let proto = Proto::from(fixture.path());
    let mut tool =
        NodeDependencyManager::new(&proto, NodeDependencyManagerType::Npm, Some("9.0.0"));

    tool.setup("9.0.0").await.unwrap();

    assert!(tool.get_install_dir().unwrap().exists());

    assert_eq!(
        tool.get_bin_path().unwrap(),
        &proto.tools_dir.join("npm/9.0.0/bin/npm-cli.js")
    );

    if cfg!(windows) {
        assert_eq!(
            tool.get_shim_path().unwrap(),
            &proto.tools_dir.join("npm\\9.0.0\\npm.bat")
        );
    } else {
        assert_eq!(
            tool.get_shim_path().unwrap(),
            &proto.tools_dir.join("npm/9.0.0/npm")
        );
    }
}

#[tokio::test]
async fn downloads_verifies_installs_pnpm() {
    let fixture = assert_fs::TempDir::new().unwrap();
    let proto = Proto::from(fixture.path());
    let mut tool =
        NodeDependencyManager::new(&proto, NodeDependencyManagerType::Pnpm, Some("7.0.0"));

    tool.setup("7.0.0").await.unwrap();

    assert!(tool.get_install_dir().unwrap().exists());

    assert_eq!(
        tool.get_bin_path().unwrap(),
        &proto.tools_dir.join("pnpm/7.0.0/bin/pnpm.cjs")
    );
}

#[tokio::test]
async fn downloads_verifies_installs_yarn_classic() {
    let fixture = assert_fs::TempDir::new().unwrap();
    let proto = Proto::from(fixture.path());
    let mut tool =
        NodeDependencyManager::new(&proto, NodeDependencyManagerType::Yarn, Some("1.22.0"));

    tool.setup("1.22.0").await.unwrap();

    assert!(tool.get_install_dir().unwrap().exists());

    assert_eq!(
        tool.get_bin_path().unwrap(),
        &proto.tools_dir.join("yarn/1.22.0/bin/yarn.js")
    );
}

#[tokio::test]
async fn downloads_verifies_installs_yarn_berry() {
    let fixture = assert_fs::TempDir::new().unwrap();
    let proto = Proto::from(fixture.path());
    let mut tool =
        NodeDependencyManager::new(&proto, NodeDependencyManagerType::Yarn, Some("3.3.0"));

    tool.setup("3.3.0").await.unwrap();

    assert!(tool.get_install_dir().unwrap().exists());

    assert_eq!(tool.get_resolved_version(), "1.22.19");
    assert_eq!(
        tool.get_bin_path().unwrap(),
        &proto.tools_dir.join("yarn/1.22.19/bin/yarn.js")
    );
}

mod detector {
    use super::*;
    use assert_fs::prelude::{FileWriteStr, PathChild};

    #[tokio::test]
    async fn doesnt_match_if_no_json_file() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let proto = Proto::from(fixture.path());
        let tool =
            NodeDependencyManager::new(&proto, NodeDependencyManagerType::Npm, Some("9.0.0"));

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn doesnt_match_if_no_field() {
        let fixture = assert_fs::TempDir::new().unwrap();

        fixture.child("package.json").write_str(r#"{}"#).unwrap();

        let proto = Proto::from(fixture.path());
        let tool =
            NodeDependencyManager::new(&proto, NodeDependencyManagerType::Npm, Some("9.0.0"));

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn doesnt_match_if_diff_package_name() {
        let fixture = assert_fs::TempDir::new().unwrap();

        fixture
            .child("package.json")
            .write_str(r#"{"packageManager":"yarn@1.2.3"}"#)
            .unwrap();

        let proto = Proto::from(fixture.path());
        let tool =
            NodeDependencyManager::new(&proto, NodeDependencyManagerType::Npm, Some("9.0.0"));

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn defaults_to_latest_version() {
        let fixture = assert_fs::TempDir::new().unwrap();

        fixture
            .child("package.json")
            .write_str(r#"{"packageManager":"npm"}"#)
            .unwrap();

        let proto = Proto::from(fixture.path());
        let tool =
            NodeDependencyManager::new(&proto, NodeDependencyManagerType::Npm, Some("9.0.0"));

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            Some("latest".into())
        );
    }

    #[tokio::test]
    async fn detects_npm() {
        let fixture = assert_fs::TempDir::new().unwrap();

        fixture
            .child("package.json")
            .write_str(r#"{"packageManager":"npm@1.2.3"}"#)
            .unwrap();

        let proto = Proto::from(fixture.path());
        let tool =
            NodeDependencyManager::new(&proto, NodeDependencyManagerType::Npm, Some("9.0.0"));

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            Some("1.2.3".into())
        );
    }

    #[tokio::test]
    async fn detects_pnpm() {
        let fixture = assert_fs::TempDir::new().unwrap();

        fixture
            .child("package.json")
            .write_str(r#"{"packageManager":"pnpm@4.5.6"}"#)
            .unwrap();

        let proto = Proto::from(fixture.path());
        let tool =
            NodeDependencyManager::new(&proto, NodeDependencyManagerType::Pnpm, Some("9.0.0"));

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            Some("4.5.6".into())
        );
    }

    #[tokio::test]
    async fn detects_yarn() {
        let fixture = assert_fs::TempDir::new().unwrap();

        fixture
            .child("package.json")
            .write_str(r#"{"packageManager":"yarn@7.8.9"}"#)
            .unwrap();

        let proto = Proto::from(fixture.path());
        let tool =
            NodeDependencyManager::new(&proto, NodeDependencyManagerType::Yarn, Some("9.0.0"));

        assert_eq!(
            tool.detect_version_from(fixture.path()).await.unwrap(),
            Some("7.8.9".into())
        );
    }
}

mod downloader {
    use super::*;

    #[tokio::test]
    async fn sets_path_to_temp() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let proto = Proto::from(fixture.path());
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
            &Proto::from(fixture.path()),
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
            &Proto::from(fixture.path()),
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
            &Proto::from(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        assert_ne!(tool.resolve_version("latest").await.unwrap(), "latest");
        assert_ne!(tool.get_resolved_version(), "latest");
    }

    #[tokio::test]
    async fn resolve_partial_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &Proto::from(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        assert_eq!(tool.resolve_version("6").await.unwrap(), "6.9.2");
    }

    #[tokio::test]
    async fn resolve_version_with_prefix() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &Proto::from(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        assert_eq!(tool.resolve_version("v9.0.0").await.unwrap(), "9.0.0");
    }

    #[tokio::test]
    async fn resolve_custom_dist() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &Proto::from(fixture.path()),
            NodeDependencyManagerType::Yarn,
            None,
        );

        assert_ne!(tool.resolve_version("berry").await.unwrap(), "berry");
    }

    #[tokio::test]
    async fn handles_npm() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &Proto::from(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        assert_eq!(tool.resolve_version("9.0.0").await.unwrap(), "9.0.0");
    }

    #[tokio::test]
    async fn handles_pnpm() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &Proto::from(fixture.path()),
            NodeDependencyManagerType::Pnpm,
            None,
        );

        assert_eq!(tool.resolve_version("7.0.0").await.unwrap(), "7.0.0");
    }
>>>
    #[tokio::test]
    async fn handles_yarn() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &Proto::from(fixture.path()),
            NodeDependencyManagerType::Yarn,
            None,
        );

        assert_eq!(tool.resolve_version("1.22.0").await.unwrap(), "1.22.0");
    }

    #[tokio::test]
    #[should_panic(expected = "VersionUnknownAlias(\"unknown\")")]
    async fn errors_invalid_alias() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &Proto::from(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        tool.resolve_version("unknown").await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "VersionResolveFailed(\"99.99.99\")")]
    async fn errors_invalid_version() {
        let fixture = assert_fs::TempDir::new().unwrap();
        let mut tool = NodeDependencyManager::new(
            &Proto::from(fixture.path()),
            NodeDependencyManagerType::Npm,
            None,
        );

        tool.resolve_version("99.99.99").await.unwrap();
    }
}
