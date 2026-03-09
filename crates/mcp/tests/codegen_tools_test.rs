use moon_mcp::tools::codegen_tools::{Generate, Variables};
use rustc_hash::FxHashMap;
use serde_json::Value;

fn assert_valid_json_schema_draft4(schema: &Value) {
    let result = jsonschema::draft4::new(schema);
    assert!(
        result.is_ok(),
        "Schema failed JSON Schema Draft 4 validation:\n{}\nSchema: {}",
        result.unwrap_err(),
        serde_json::to_string_pretty(schema).unwrap(),
    );
}

fn assert_valid_json_schema_draft6(schema: &Value) {
    let result = jsonschema::draft6::new(schema);
    assert!(
        result.is_ok(),
        "Schema failed JSON Schema Draft 6 validation:\n{}\nSchema: {}",
        result.unwrap_err(),
        serde_json::to_string_pretty(schema).unwrap(),
    );
}

fn assert_valid_json_schema_draft7(schema: &Value) {
    let result = jsonschema::draft7::new(schema);
    assert!(
        result.is_ok(),
        "Schema failed JSON Schema Draft 7 validation:\n{}\nSchema: {}",
        result.unwrap_err(),
        serde_json::to_string_pretty(schema).unwrap(),
    );
}

fn assert_valid_json_schema_draft201909(schema: &Value) {
    let result = jsonschema::draft201909::new(schema);
    assert!(
        result.is_ok(),
        "Schema failed JSON Schema Draft 2019-09 validation:\n{}\nSchema: {}",
        result.unwrap_err(),
        serde_json::to_string_pretty(schema).unwrap(),
    );
}

fn assert_valid_json_schema_draft202012(schema: &Value) {
    let result = jsonschema::draft202012::new(schema);
    assert!(
        result.is_ok(),
        "Schema failed JSON Schema Draft 2020-12 validation:\n{}\nSchema: {}",
        result.unwrap_err(),
        serde_json::to_string_pretty(schema).unwrap(),
    );
}

/// Validate a schema against all commonly used JSON Schema drafts.
fn assert_valid_json_schema_all_drafts(schema: &Value) {
    assert_valid_json_schema_draft4(schema);
    assert_valid_json_schema_draft6(schema);
    assert_valid_json_schema_draft7(schema);
    assert_valid_json_schema_draft201909(schema);
    assert_valid_json_schema_draft202012(schema);
}

mod codegen_tools {
    use super::*;

    mod variables {
        use super::*;

        #[test]
        fn schema_is_valid_all_drafts() {
            let schema = Value::Object(Variables::json_schema());
            assert_valid_json_schema_all_drafts(&schema);
        }

        #[test]
        fn serde_roundtrip() {
            let mut map = FxHashMap::default();
            map.insert("name".to_string(), Value::String("test".to_string()));
            map.insert("count".to_string(), Value::Number(42.into()));
            let vars = Variables(map);

            let json = serde_json::to_string(&vars).unwrap();
            let deserialized: Variables = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.len(), 2);
            assert_eq!(deserialized.get("name").unwrap(), "test");
        }
    }

    mod generate {
        use super::*;

        #[test]
        fn schema_is_valid_all_drafts() {
            let schema = Value::Object(Generate::json_schema());
            assert_valid_json_schema_all_drafts(&schema);
        }

        #[test]
        fn tool_input_schema_is_valid_all_drafts() {
            let tool = Generate::tool();
            let schema = serde_json::to_value(&tool.input_schema).unwrap();
            assert_valid_json_schema_all_drafts(&schema);
        }

        #[test]
        fn tool_input_schema_validates_sample_input() {
            let tool = Generate::tool();
            let schema = serde_json::to_value(&tool.input_schema).unwrap();
            let validator = jsonschema::draft202012::new(&schema).unwrap();

            let valid_input = serde_json::json!({
                "template": "my-template",
                "to": "./output",
                "dry_run": true,
                "variables": {"name": "test", "count": 42}
            });
            let result = validator.validate(&valid_input);
            assert!(
                result.is_ok(),
                "Valid input rejected: {}",
                result.unwrap_err(),
            );
        }

        #[test]
        fn tool_input_schema_rejects_missing_required() {
            let tool = Generate::tool();
            let schema = serde_json::to_value(&tool.input_schema).unwrap();
            let validator = jsonschema::draft202012::new(&schema).unwrap();

            // Missing required "template" and "to" fields
            let invalid_input = serde_json::json!({"dry_run": true});
            assert!(
                validator.validate(&invalid_input).is_err(),
                "Schema should reject input missing required fields"
            );
        }
    }
}
