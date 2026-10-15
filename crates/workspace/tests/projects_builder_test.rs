use moon_common::Id;
use moon_config::{LanguageType, LayerType, ProjectConfig, StackType, WorkspaceProjects};
use moon_config_loader::{ConfigLoader, ProjectConfigFile};
use moon_test_utils::WorkspaceMocker;
use moon_workspace::WorkspaceBuilder;
use rustc_hash::FxHashMap;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::path::Path;

fn create_loader(root: &Path) -> ConfigLoader {
    ConfigLoader::new(root.join(".moon"))
}

fn create_mocker(root: &Path, sources: &[(&str, &str)]) -> WorkspaceMocker {
    let sources = sources
        .iter()
        .map(|(id, source)| (Id::raw(id), source.to_string()))
        .collect::<FxHashMap<_, _>>();

    WorkspaceMocker::new(root).update_workspace_config(move |config| {
        config.projects = WorkspaceProjects::Sources(sources.clone());
    })
}

async fn build_error(mocker: &WorkspaceMocker) -> String {
    let mut builder = WorkspaceBuilder::new(mocker.mock_workspace_builder_context())
        .await
        .unwrap();

    let result = match builder.load_graphs().await {
        Ok(_) => builder.build().await.map(|_| ()),
        Err(error) => Err(error),
    };

    result.unwrap_err().to_string()
}

fn create_multi_config_sandbox() -> Sandbox {
    let sandbox = create_empty_sandbox();

    // Created out of precedence order, so that directory order can't line up
    sandbox.create_file("project/moon.toml", "language = 'python'");
    sandbox.create_file("project/moon.json", r#"{ "layer": "tool" }"#);
    sandbox.create_file("project/moon.yaml", "language: 'go'\nlayer: 'library'");
    sandbox.create_file("project/moon.yml", "language: 'rust'\nstack: 'backend'");
    sandbox.create_file("project/other.yml", "language: 'ruby'");
    sandbox
}

mod locate_project_file {
    use super::*;

    #[test]
    fn missing_root() {
        let sandbox = create_empty_sandbox();
        let loader = create_loader(sandbox.path());

        assert_eq!(
            loader.locate_project_file(&sandbox.path().join("missing")),
            ProjectConfigFile::MissingRoot
        );
    }

    #[test]
    fn no_config_file() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("project/file.txt", "");

        let loader = create_loader(sandbox.path());

        assert_eq!(
            loader.locate_project_file(&sandbox.path().join("project")),
            ProjectConfigFile::Located(None)
        );
    }

    #[test]
    fn single_config_file() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("project/moon.toml", "");

        let loader = create_loader(sandbox.path());
        let root = sandbox.path().join("project");

        assert_eq!(
            loader.locate_project_file(&root),
            ProjectConfigFile::Located(Some(root.join("moon.toml")))
        );
    }

    #[test]
    fn uses_extension_precedence() {
        let sandbox = create_multi_config_sandbox();
        let loader = create_loader(sandbox.path());
        let root = sandbox.path().join("project");

        assert_eq!(
            loader.locate_project_file(&root),
            ProjectConfigFile::Located(Some(root.join("moon.yml")))
        );

        std::fs::remove_file(root.join("moon.yml")).unwrap();

        assert_eq!(
            loader.locate_project_file(&root),
            ProjectConfigFile::Located(Some(root.join("moon.yaml")))
        );

        std::fs::remove_file(root.join("moon.yaml")).unwrap();

        assert_eq!(
            loader.locate_project_file(&root),
            ProjectConfigFile::Located(Some(root.join("moon.json")))
        );

        std::fs::remove_file(root.join("moon.json")).unwrap();

        assert_eq!(
            loader.locate_project_file(&root),
            ProjectConfigFile::Located(Some(root.join("moon.toml")))
        );
    }

    #[test]
    fn root_level_source() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("moon.yaml", "");
        sandbox.create_file("project/moon.yml", "");

        let loader = create_loader(sandbox.path());
        let root = sandbox.path().join(".");

        assert_eq!(
            loader.locate_project_file(&root),
            ProjectConfigFile::Located(Some(root.join("moon.yaml")))
        );
    }

    #[test]
    fn file_source() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("project", "");

        let loader = create_loader(sandbox.path());

        assert_eq!(
            loader.locate_project_file(&sandbox.path().join("project")),
            ProjectConfigFile::Located(None)
        );
    }
}

mod load_project_config_from_file {
    use super::*;

    #[test]
    fn loads_defaults_without_file() {
        let sandbox = create_empty_sandbox();
        std::fs::create_dir_all(sandbox.path().join("project")).unwrap();

        let loader = create_loader(sandbox.path());
        let config = loader
            .load_project_config_from_file(sandbox.path(), None)
            .unwrap();

        assert_eq!(
            config,
            loader
                .load_project_config_from_source(sandbox.path(), "project")
                .unwrap()
        );
        assert_eq!(config, ProjectConfig::default());
    }

    #[test]
    fn loads_only_the_located_file() {
        let sandbox = create_multi_config_sandbox();
        let loader = create_loader(sandbox.path());

        let ProjectConfigFile::Located(file) =
            loader.locate_project_file(&sandbox.path().join("project"))
        else {
            panic!("Expected a located file");
        };

        let config = loader
            .load_project_config_from_file(sandbox.path(), file.as_deref())
            .unwrap();

        assert_eq!(config.language, LanguageType::Rust);
        assert_eq!(config.layer, LayerType::Unknown);
        assert_eq!(config.stack, StackType::Backend);
    }

    #[test]
    fn errors_with_the_same_message_as_probing() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("project/moon.yml", "language: [unclosed");

        let loader = create_loader(sandbox.path());

        // Join the segments separately, so that the path uses the platform
        // separator, like the path that probing produces
        let file = sandbox.path().join("project").join("moon.yml");

        let error = loader
            .load_project_config_from_file(sandbox.path(), Some(&file))
            .unwrap_err();
        let probe_error = loader
            .load_project_config_from_source(sandbox.path(), "project")
            .unwrap_err();

        assert_eq!(format!("{error:?}"), format!("{probe_error:?}"));
    }
}

mod projects_builder {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_for_missing_source() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/moon.yml", "");

        let mocker = create_mocker(sandbox.path(), &[("a", "a"), ("b", "missing/b")]);

        assert_eq!(
            build_error(&mocker).await,
            "No project exists at source path missing/b."
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_error_for_missing_source_when_not_requested() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/moon.yml", "");

        let graph = create_mocker(sandbox.path(), &[("a", "a"), ("b", "missing/b")])
            .mock_workspace_graph_for(&["a"])
            .await;

        assert!(graph.get_project("a").is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_defaults_without_config() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/file.txt", "");

        let graph = create_mocker(sandbox.path(), &[("a", "a")])
            .mock_workspace_graph()
            .await;

        assert_eq!(
            graph.get_project("a").unwrap().config,
            ProjectConfig::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_highest_precedence_config() {
        let sandbox = create_multi_config_sandbox();

        let graph = create_mocker(sandbox.path(), &[("project", "project")])
            .mock_workspace_graph()
            .await;

        let config = &graph.get_project("project").unwrap().config;

        assert_eq!(config.language, LanguageType::Rust);
        assert_eq!(config.layer, LayerType::Unknown);
        assert_eq!(config.stack, StackType::Backend);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_root_level_project() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("moon.yml", "language: 'rust'");
        sandbox.create_file("a/moon.yml", "language: 'go'");
        sandbox.create_file("b/file.txt", "");

        let graph = create_mocker(sandbox.path(), &[("root", "."), ("a", "a"), ("b", "b")])
            .mock_workspace_graph()
            .await;

        assert_eq!(
            graph.get_project("root").unwrap().config.language,
            LanguageType::Rust
        );
        assert_eq!(
            graph.get_project("a").unwrap().config.language,
            LanguageType::Go
        );
        assert_eq!(
            graph.get_project("b").unwrap().config,
            ProjectConfig::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_file_names_like_probing() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a/Moon.yml", "language: 'rust'");

        let graph = create_mocker(sandbox.path(), &[("a", "a")])
            .mock_workspace_graph()
            .await;

        // Depends on the file system's case sensitivity, which is the same
        // for both, as each probes for the lowercase file name
        assert_eq!(
            graph.get_project("a").unwrap().config,
            create_loader(sandbox.path())
                .load_project_config_from_source(sandbox.path(), "a")
                .unwrap()
        );
    }
}
