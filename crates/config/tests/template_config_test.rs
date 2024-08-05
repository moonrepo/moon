mod utils;

use moon_common::consts::CONFIG_TEMPLATE_FILENAME_YML;
use moon_config::{TemplateConfig, TemplateVariableEnumDefault};
use rustc_hash::FxHashMap;
use utils::*;

mod template_config {
    use super::*;

    #[test]
    #[should_panic(
        expected = "unknown field `unknown`, expected one of `$schema`, `description`, `destination`, `extends`, `id`, `title`, `variables`"
    )]
    fn error_unknown_field() {
        test_load_config(CONFIG_TEMPLATE_FILENAME_YML, "unknown: 123", |path| {
            TemplateConfig::load_from(path)
        });
    }

    #[test]
    fn loads_defaults() {
        let config = test_load_config(
            CONFIG_TEMPLATE_FILENAME_YML,
            "title: title\ndescription: description",
            |path| TemplateConfig::load_from(path),
        );

        assert_eq!(config.title, "title");
        assert_eq!(config.description, "description");
        assert_eq!(config.variables, FxHashMap::default());
    }

    mod title {
        use super::*;

        #[test]
        #[should_panic(expected = "invalid type: integer `123`, expected a string")]
        fn invalid_type() {
            test_load_config(CONFIG_TEMPLATE_FILENAME_YML, "title: 123", |path| {
                TemplateConfig::load_from(path)
            });
        }

        #[test]
        #[should_panic(expected = "title: must not be empty")]
        fn not_empty() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                "title: ''\ndescription: 'asd'",
                |path| TemplateConfig::load_from(path),
            );
        }
    }

    mod description {
        use super::*;

        #[test]
        #[should_panic(expected = "invalid type: integer `123`, expected a string")]
        fn invalid_type() {
            test_load_config(CONFIG_TEMPLATE_FILENAME_YML, "description: 123", |path| {
                TemplateConfig::load_from(path)
            });
        }

        #[test]
        #[should_panic(expected = "description: must not be empty")]
        fn not_empty() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                "title: 'asd'\ndescription: ''",
                |path| TemplateConfig::load_from(path),
            );
        }
    }

    mod variables {
        use super::*;
        use moon_config::{
            TemplateVariable, TemplateVariableBoolSetting, TemplateVariableEnumSetting,
            TemplateVariableEnumValue, TemplateVariableEnumValueConfig,
            TemplateVariableNumberSetting, TemplateVariableStringSetting,
        };

        #[test]
        #[should_panic(
            expected = "unknown variant `array`, expected one of `boolean`, `enum`, `number`, `string`"
        )]
        fn error_unknown_variable_type() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
  unknown:
    type: array
",
                |path| TemplateConfig::load_from(path),
            );
        }

        #[test]
        fn loads_boolean() {
            let config = test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
  bool:
    type: boolean
    default: false
    prompt: prompt
    required: true
",
                |path| TemplateConfig::load_from(path),
            );

            assert_eq!(
                *config.variables.get("bool").unwrap(),
                TemplateVariable::Boolean(TemplateVariableBoolSetting {
                    default: false,
                    internal: false,
                    order: None,
                    prompt: Some("prompt".into()),
                    required: Some(true)
                })
            );
        }

        #[test]
        #[should_panic(expected = "invalid type: integer `123`, expected a boolean")]
        fn invalid_boolean() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
  bool:
    type: boolean
    default: 123
",
                |path| TemplateConfig::load_from(path),
            );
        }

        #[test]
        fn loads_number() {
            let config = test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
  num:
    type: number
    default: 123
    prompt: prompt
    required: false
",
                |path| TemplateConfig::load_from(path),
            );

            assert_eq!(
                *config.variables.get("num").unwrap(),
                TemplateVariable::Number(TemplateVariableNumberSetting {
                    default: 123,
                    internal: false,
                    order: None,
                    prompt: Some("prompt".into()),
                    required: Some(false)
                })
            );
        }

        #[test]
        #[should_panic(expected = "invalid type: boolean `true`, expected isize")]
        fn invalid_number() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
  num:
    type: number
    default: true
",
                |path| TemplateConfig::load_from(path),
            );
        }

        #[test]
        fn loads_string() {
            let config = test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
  str:
    type: string
    default: abc
",
                |path| TemplateConfig::load_from(path),
            );

            assert_eq!(
                *config.variables.get("str").unwrap(),
                TemplateVariable::String(TemplateVariableStringSetting {
                    default: "abc".into(),
                    internal: false,
                    order: None,
                    prompt: None,
                    required: None
                })
            );
        }

        #[test]
        #[should_panic(expected = "invalid type: integer `123`, expected a string")]
        fn invalid_string() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
  str:
    type: string
    default: 123
",
                |path| TemplateConfig::load_from(path),
            );
        }

        #[test]
        fn loads_string_enum() {
            let config = test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
  strum:
    type: enum
    default: a
    values:
      - a
      - b
      - label: C
        value: c
    prompt: prompt
",
                |path| TemplateConfig::load_from(path),
            );

            assert_eq!(
                *config.variables.get("strum").unwrap(),
                TemplateVariable::Enum(TemplateVariableEnumSetting {
                    default: TemplateVariableEnumDefault::String("a".into()),
                    internal: false,
                    multiple: None,
                    order: None,
                    prompt: Some("prompt".into()),
                    values: vec![
                        TemplateVariableEnumValue::String("a".into()),
                        TemplateVariableEnumValue::String("b".into()),
                        TemplateVariableEnumValue::Object(TemplateVariableEnumValueConfig {
                            label: "C".into(),
                            value: "c".into()
                        })
                    ],
                })
            );
        }

        #[test]
        fn loads_string_enum_default_list() {
            let config = test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
  strum:
    type: enum
    default:
      - a
      - c
    values:
      - a
      - b
      - label: C
        value: c
    multiple: true
    prompt: prompt
",
                |path| TemplateConfig::load_from(path),
            );

            assert_eq!(
                *config.variables.get("strum").unwrap(),
                TemplateVariable::Enum(TemplateVariableEnumSetting {
                    default: TemplateVariableEnumDefault::Vec(vec!["a".into(), "c".into()]),
                    internal: false,
                    multiple: Some(true),
                    order: None,
                    prompt: Some("prompt".into()),
                    values: vec![
                        TemplateVariableEnumValue::String("a".into()),
                        TemplateVariableEnumValue::String("b".into()),
                        TemplateVariableEnumValue::Object(TemplateVariableEnumValueConfig {
                            label: "C".into(),
                            value: "c".into()
                        })
                    ],
                })
            );
        }

        #[test]
        #[should_panic(expected = "expected a value string or value object with label")]
        fn invalid_string_enum() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
   strum:
    type: enum
    default: c
    values: [1, 2, 3]
    prompt: prompt
",
                |path| TemplateConfig::load_from(path),
            );
        }

        #[test]
        #[should_panic(expected = "invalid default value, must be a value configured in `values`")]
        fn invalid_enum_default_value() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
   strum:
    type: enum
    default: z
    values: [a, b, c]
    prompt: prompt
",
                |path| TemplateConfig::load_from(path),
            );
        }

        #[test]
        #[should_panic(
            expected = "multiple default values is not allowed unless `multiple` is enabled"
        )]
        fn errors_multi_default_when_not_multiple() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME_YML,
                r"
title: title
description: description
variables:
   strum:
    type: enum
    default: [a, b]
    values: [a, b, c]
    prompt: prompt
",
                |path| TemplateConfig::load_from(path),
            );
        }
    }
}
