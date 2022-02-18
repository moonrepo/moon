use moon_config::GlobalProjectConfig;
use moon_project::ProjectGraph;
use moon_utils::string_vec;
use moon_utils::test::get_fixtures_dir;
use std::collections::HashMap;

mod get_dependencies_of {
    use super::*;

    #[test]
    fn returns_dep_list() {
        let workspace_root = get_fixtures_dir("project-graph/dependencies");
        let graph = ProjectGraph::new(
            &workspace_root,
            GlobalProjectConfig::default(),
            &HashMap::from([
                ("a".to_owned(), "a".to_owned()),
                ("b".to_owned(), "b".to_owned()),
                ("c".to_owned(), "c".to_owned()),
                ("d".to_owned(), "d".to_owned()),
            ]),
        );

        let a = graph.get("a").unwrap();
        let b = graph.get("b").unwrap();
        let c = graph.get("c").unwrap();
        let d = graph.get("d").unwrap();

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
        let workspace_root = get_fixtures_dir("project-graph/dependents");
        let graph = ProjectGraph::new(
            &workspace_root,
            GlobalProjectConfig::default(),
            &HashMap::from([
                ("a".to_owned(), "a".to_owned()),
                ("b".to_owned(), "b".to_owned()),
                ("c".to_owned(), "c".to_owned()),
                ("d".to_owned(), "d".to_owned()),
            ]),
        );

        let a = graph.get("a").unwrap();
        let b = graph.get("b").unwrap();
        let c = graph.get("c").unwrap();
        let d = graph.get("d").unwrap();

        assert_eq!(graph.get_dependents_of(&a).unwrap(), string_vec![]);
        assert_eq!(graph.get_dependents_of(&b).unwrap(), string_vec!["a"]);
        assert_eq!(graph.get_dependents_of(&c).unwrap(), string_vec!["b"]);
        assert_eq!(
            graph.get_dependents_of(&d).unwrap(),
            string_vec!["a", "b", "c"]
        );
    }
}
