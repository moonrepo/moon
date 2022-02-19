use insta::assert_snapshot;
use moon_config::GlobalProjectConfig;
use moon_project::ProjectGraph;
use moon_utils::string_vec;
use moon_utils::test::get_fixtures_dir;
use std::collections::HashMap;

fn get_dependencies_graph() -> ProjectGraph {
    let workspace_root = get_fixtures_dir("project-graph/dependencies");

    ProjectGraph::new(
        &workspace_root,
        GlobalProjectConfig::default(),
        &HashMap::from([
            ("a".to_owned(), "a".to_owned()),
            ("b".to_owned(), "b".to_owned()),
            ("c".to_owned(), "c".to_owned()),
            ("d".to_owned(), "d".to_owned()),
        ]),
    )
}

fn get_dependents_graph() -> ProjectGraph {
    let workspace_root = get_fixtures_dir("project-graph/dependents");

    ProjectGraph::new(
        &workspace_root,
        GlobalProjectConfig::default(),
        &HashMap::from([
            ("a".to_owned(), "a".to_owned()),
            ("b".to_owned(), "b".to_owned()),
            ("c".to_owned(), "c".to_owned()),
            ("d".to_owned(), "d".to_owned()),
        ]),
    )
}

mod get_dependencies_of {
    use super::*;

    #[test]
    fn returns_dep_list() {
        let graph = get_dependencies_graph();

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

    #[test]
    fn returns_dep_list() {
        let graph = get_dependents_graph();

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

    #[test]
    fn renders_tree() {
        let graph = get_dependencies_graph();

        graph.load("a").unwrap();
        graph.load("b").unwrap();
        graph.load("c").unwrap();
        graph.load("d").unwrap();

        assert_snapshot!(graph.to_dot());
    }
}
