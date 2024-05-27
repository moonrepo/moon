use moon_codegen::parse_args_into_variables;
use moon_config::{
    TemplateVariable, TemplateVariableBoolSetting, TemplateVariableEnumSetting,
    TemplateVariableEnumValue, TemplateVariableNumberSetting, TemplateVariableStringSetting,
};
use rustc_hash::FxHashMap;
use tera::Number;
use tera::Value;

mod cli_args {
    use super::*;

    fn create_vars() -> FxHashMap<String, TemplateVariable> {
        let mut vars = FxHashMap::default();
        vars.insert(
            "internal".into(),
            TemplateVariable::Boolean(TemplateVariableBoolSetting {
                internal: true,
                ..Default::default()
            }),
        );
        vars.insert(
            "bool".into(),
            TemplateVariable::Boolean(TemplateVariableBoolSetting::default()),
        );
        vars.insert(
            "number".into(),
            TemplateVariable::Number(TemplateVariableNumberSetting::default()),
        );
        vars.insert(
            "string".into(),
            TemplateVariable::String(TemplateVariableStringSetting::default()),
        );
        vars.insert(
            "enum".into(),
            TemplateVariable::Enum(TemplateVariableEnumSetting {
                values: vec![
                    TemplateVariableEnumValue::String("a".into()),
                    TemplateVariableEnumValue::String("b".into()),
                    TemplateVariableEnumValue::String("c".into()),
                ],
                ..Default::default()
            }),
        );
        vars.insert(
            "multienum".into(),
            TemplateVariable::Enum(TemplateVariableEnumSetting {
                multiple: Some(true),
                values: vec![
                    TemplateVariableEnumValue::String("a".into()),
                    TemplateVariableEnumValue::String("b".into()),
                    TemplateVariableEnumValue::String("c".into()),
                ],
                ..Default::default()
            }),
        );
        vars
    }

    #[test]
    #[should_panic(expected = "unexpected argument")]
    fn errors_if_internal_arg_passed() {
        let context = parse_args_into_variables(&["--internal".into()], &create_vars()).unwrap();

        assert!(!context.contains_key("internal"));
    }

    mod bool {
        use super::*;

        #[test]
        fn nothing_when_no_matching_arg() {
            let context = parse_args_into_variables(&[], &create_vars()).unwrap();

            assert!(!context.contains_key("bool"));
        }

        #[test]
        fn sets_truthy_var() {
            let context = parse_args_into_variables(&["--bool".into()], &create_vars()).unwrap();

            assert_eq!(context.get("bool").unwrap(), &Value::Bool(true));
        }

        #[test]
        fn sets_falsy_var() {
            let context = parse_args_into_variables(&["--no-bool".into()], &create_vars()).unwrap();

            assert_eq!(context.get("bool").unwrap(), &Value::Bool(false));
        }

        #[test]
        fn can_overwrite_truthy() {
            let context =
                parse_args_into_variables(&["--bool".into(), "--no-bool".into()], &create_vars())
                    .unwrap();

            assert_eq!(context.get("bool").unwrap(), &Value::Bool(false));
        }

        #[test]
        fn can_overwrite_falsy() {
            let context =
                parse_args_into_variables(&["--no-bool".into(), "--bool".into()], &create_vars())
                    .unwrap();

            assert_eq!(context.get("bool").unwrap(), &Value::Bool(true));
        }
    }

    mod number {
        use super::*;

        #[test]
        fn nothing_when_no_matching_arg() {
            let context = parse_args_into_variables(&[], &create_vars()).unwrap();

            assert!(!context.contains_key("number"));
        }

        #[test]
        fn sets_var() {
            let context =
                parse_args_into_variables(&["--number".into(), "123".into()], &create_vars())
                    .unwrap();

            assert_eq!(
                context.get("number").unwrap(),
                &Value::Number(Number::from(123))
            );
        }

        #[test]
        fn sets_negative_var() {
            let context =
                parse_args_into_variables(&["--number".into(), "-123".into()], &create_vars())
                    .unwrap();

            assert_eq!(
                context.get("number").unwrap(),
                &Value::Number(Number::from(-123))
            );
        }

        #[test]
        #[should_panic(expected = "a value is required")]
        fn errors_when_no_value() {
            parse_args_into_variables(&["--number".into()], &create_vars()).unwrap();
        }
    }

    mod string {
        use super::*;

        #[test]
        fn nothing_when_no_matching_arg() {
            let context = parse_args_into_variables(&[], &create_vars()).unwrap();

            assert!(!context.contains_key("string"));
        }

        #[test]
        fn sets_var() {
            let context =
                parse_args_into_variables(&["--string".into(), "abc".into()], &create_vars())
                    .unwrap();

            assert_eq!(context.get("string").unwrap(), &Value::String("abc".into()));
        }

        #[test]
        #[should_panic(expected = "a value is required")]
        fn errors_when_no_value() {
            parse_args_into_variables(&["--string".into()], &create_vars()).unwrap();
        }
    }

    mod enum_single {
        use super::*;

        #[test]
        fn nothing_when_no_matching_arg() {
            let context = parse_args_into_variables(&[], &create_vars()).unwrap();

            assert!(!context.contains_key("enum"));
        }

        #[test]
        fn sets_var() {
            let context =
                parse_args_into_variables(&["--enum".into(), "a".into()], &create_vars()).unwrap();

            assert_eq!(context.get("enum").unwrap(), &Value::String("a".into()));
        }

        #[test]
        #[should_panic(expected = "a value is required")]
        fn errors_when_no_value() {
            parse_args_into_variables(&["--enum".into()], &create_vars()).unwrap();
        }

        #[test]
        #[should_panic(expected = "invalid value 'd'")]
        fn errors_if_wrong_value() {
            parse_args_into_variables(&["--enum".into(), "d".into()], &create_vars()).unwrap();
        }
    }

    mod enum_multiple {
        use super::*;

        #[test]
        fn nothing_when_no_matching_arg() {
            let context = parse_args_into_variables(&[], &create_vars()).unwrap();

            assert!(!context.contains_key("multienum"));
        }

        #[test]
        fn sets_var() {
            let context =
                parse_args_into_variables(&["--multienum".into(), "a".into()], &create_vars())
                    .unwrap();

            assert_eq!(
                context.get("multienum").unwrap(),
                &Value::Array(vec![Value::String("a".into())])
            );
        }

        #[test]
        fn sets_var_with_many() {
            let context = parse_args_into_variables(
                &[
                    "--multienum".into(),
                    "a".into(),
                    "--multienum".into(),
                    "b".into(),
                ],
                &create_vars(),
            )
            .unwrap();

            assert_eq!(
                context.get("multienum").unwrap(),
                &Value::Array(vec![Value::String("a".into()), Value::String("b".into())])
            );
        }

        #[test]
        #[should_panic(expected = "a value is required")]
        fn errors_when_no_value() {
            parse_args_into_variables(&["--multienum".into()], &create_vars()).unwrap();
        }

        #[test]
        #[should_panic(expected = "invalid value 'd'")]
        fn errors_if_wrong_value() {
            parse_args_into_variables(&["--multienum".into(), "d".into()], &create_vars()).unwrap();
        }
    }
}
