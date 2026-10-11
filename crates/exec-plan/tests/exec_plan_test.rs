use moon_affected::Depth;
use moon_exec_plan::*;

fn parse(json: &str) -> ExecutionPlan {
    serde_json::from_str(json).unwrap()
}

mod graph_block {
    use super::*;

    #[test]
    fn parses_dependents_and_no_deps() {
        let plan = parse(r#"{ "graph": { "dependents": "direct", "noDeps": true } }"#);

        assert_eq!(plan.graph.dependents, Some(Depth::DIRECT));
        assert_eq!(plan.graph.no_deps, Some(true));
        assert_eq!(plan.graph.downstream, None);
        assert_eq!(plan.graph.upstream, None);
    }

    #[test]
    fn parses_numeric_dependents() {
        let plan = parse(r#"{ "graph": { "dependents": 3 } }"#);

        assert_eq!(plan.graph.dependents, Some(Depth::Limit(3)));
    }

    #[test]
    fn parses_deprecated_scopes() {
        let plan = parse(r#"{ "graph": { "downstream": "deep", "upstream": "none" } }"#);

        assert_eq!(plan.graph.dependents, None);
        assert_eq!(plan.graph.downstream, Some(Depth::Deep));
        assert_eq!(plan.graph.upstream, Some(Depth::NONE));
    }

    #[test]
    fn doesnt_serialize_deprecated_scopes() {
        let plan = parse(
            r#"{ "graph": { "dependents": 2, "downstream": "deep", "upstream": "direct" } }"#,
        );
        let json = serde_json::to_value(&plan.graph).unwrap();

        assert_eq!(json, serde_json::json!({ "dependents": 2, "noDeps": null }));
    }

    #[test]
    #[should_panic(expected = "unknown field `deep`")]
    fn errors_for_unknown_fields() {
        parse(r#"{ "graph": { "deep": true } }"#);
    }
}

mod affected_block {
    use super::*;

    #[test]
    fn parses_depth() {
        let plan = parse(r#"{ "affected": { "depth": "deep" } }"#);
        let affected = plan.affected.unwrap();

        assert_eq!(affected.depth, Some(Depth::Deep));
        assert_eq!(affected.include_relations, None);
    }

    #[test]
    fn parses_deprecated_include_relations() {
        let plan = parse(r#"{ "affected": { "includeRelations": true } }"#);
        let affected = plan.affected.unwrap();

        assert_eq!(affected.depth, None);
        assert_eq!(affected.include_relations, Some(true));
    }

    #[test]
    fn doesnt_serialize_deprecated_include_relations() {
        let plan = parse(r#"{ "affected": { "depth": 0, "includeRelations": true } }"#);
        let json = serde_json::to_value(plan.affected.unwrap()).unwrap();

        assert_eq!(json["depth"], "none");
        assert!(json.get("includeRelations").is_none());
    }
}
