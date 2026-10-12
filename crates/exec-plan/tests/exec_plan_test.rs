use moon_affected::Depth;
use moon_exec_plan::*;

fn parse(json: &str) -> ExecutionPlan {
    serde_json::from_str(json).unwrap()
}

mod graph_block {
    use super::*;

    #[test]
    fn parses_dependents_and_no_dependencies() {
        let plan = parse(r#"{ "graph": { "dependents": "direct", "noDependencies": true } }"#);

        assert_eq!(plan.graph.dependents, Some(Depth::DIRECT));
        assert_eq!(plan.graph.no_dependencies, Some(true));
    }

    #[test]
    fn parses_numeric_dependents() {
        let plan = parse(r#"{ "graph": { "dependents": 3 } }"#);

        assert_eq!(plan.graph.dependents, Some(Depth::Limit(3)));
    }

    #[test]
    fn serializes_fields() {
        let plan = parse(r#"{ "graph": { "dependents": 2, "noDependencies": false } }"#);
        let json = serde_json::to_value(&plan.graph).unwrap();

        assert_eq!(
            json,
            serde_json::json!({ "dependents": 2, "noDependencies": false })
        );
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
    }

    #[test]
    fn serializes_depth() {
        let plan = parse(r#"{ "affected": { "depth": 0 } }"#);
        let json = serde_json::to_value(plan.affected.unwrap()).unwrap();

        assert_eq!(json["depth"], "none");
    }
}
