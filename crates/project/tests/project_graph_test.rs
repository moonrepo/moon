use insta::assert_snapshot;
use moon_cache::CacheEngine;
use moon_config::{GlobalProjectConfig, WorkspaceConfig};
use moon_project::ProjectGraph;
use moon_utils::string_vec;
use moon_utils::test::get_fixtures_dir;
use std::collections::HashMap;

async fn get_dependencies_graph() -> ProjectGraph {
    let workspace_root = get_fixtures_dir("project-graph/dependencies");
    let workspace_config = WorkspaceConfig {
        projects: HashMap::from([
            ("a".to_owned(), "a".to_owned()),
            ("b".to_owned(), "b".to_owned()),
            ("c".to_owned(), "c".to_owned()),
            ("d".to_owned(), "d".to_owned()),
        ]),
        ..WorkspaceConfig::default()
    };

    ProjectGraph::create(
        &workspace_root,
        &workspace_config,
        GlobalProjectConfig::default(),
        &CacheEngine::create(&workspace_root).await.unwrap(),
    )
    .await
    .unwrap()
}

async fn get_dependents_graph() -> ProjectGraph {
    let workspace_root = get_fixtures_dir("project-graph/dependents");
    let workspace_config = WorkspaceConfig {
        projects: HashMap::from([
            ("a".to_owned(), "a".to_owned()),
            ("b".to_owned(), "b".to_owned()),
            ("c".to_owned(), "c".to_owned()),
            ("d".to_owned(), "d".to_owned()),
        ]),
        ..WorkspaceConfig::default()
    };

    ProjectGraph::create(
        &workspace_root,
        &workspace_config,
        GlobalProjectConfig::default(),
        &CacheEngine::create(&workspace_root).await.unwrap(),
    )
    .await
    .unwrap()
}

mod get_dependencies_of {
    use super::*;

    #[tokio::test]
    async fn returns_dep_list() {
        let graph = get_dependencies_graph().await;

        let a = graph.load("a").unwrap();
        let b = graph.load("b").unwrap();
        let c = graph.load("c").unwrap();
        let d = graph.load("d").unwrap();

        assert_eq!(graph.get_dependencies_of(&a).unwrap(), string_vec!["b"]);
        assert_eq!(graph.get_dependencies_of(&b).unwrap(), string_vec!["c"]);
        assert_eq!(graph.get_dependencies_of(&c).unwrap(), string_vec![]);
        assert_eq!(
            graph.get_dependencies_of(&d).unwrap(),
            string_vec!["c", "b", "a"]
        );
    }
}

mod get_dependents_of {
    use super::*;

    #[tokio::test]
    async fn returns_dep_list() {
        let graph = get_dependents_graph().await;

        let a = graph.load("a").unwrap();
        let b = graph.load("b").unwrap();
        let c = graph.load("c").unwrap();
        let d = graph.load("d").unwrap();

        assert_eq!(graph.get_dependents_of(&a).unwrap(), string_vec![]);
        assert_eq!(graph.get_dependents_of(&b).unwrap(), string_vec!["a"]);
        assert_eq!(graph.get_dependents_of(&c).unwrap(), string_vec!["b"]);
        assert_eq!(
            graph.get_dependents_of(&d).unwrap(),
            string_vec!["a", "b", "c"]
        );
    }
}

mod to_dot {
    use super::*;

    #[tokio::test]
    async fn renders_tree() {
        let graph = get_dependencies_graph().await;

        graph.load("a").unwrap();
        graph.load("b").unwrap();
        graph.load("c").unwrap();
        graph.load("d").unwrap();

        assert_snapshot!(graph.to_dot());
    }
}
