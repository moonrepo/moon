mod utils;

use moon_common::consts::CONFIG_TEMPLATE_FILENAME;
use moon_config2::TemplateConfig;
use rustc_hash::FxHashMap;
use utils::*;

mod template_config {
    use super::*;

    #[test]
    fn loads_defaults() {
        let config = test_load_config(
            CONFIG_TEMPLATE_FILENAME,
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
            test_load_config(CONFIG_TEMPLATE_FILENAME, "title: 123", |path| {
                TemplateConfig::load_from(path)
            });
        }

        #[test]
        #[should_panic(expected = "title: must not be empty")]
        fn not_empty() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME,
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
            test_load_config(CONFIG_TEMPLATE_FILENAME, "description: 123", |path| {
                TemplateConfig::load_from(path)
            });
        }

        #[test]
        #[should_panic(expected = "description: must not be empty")]
        fn not_empty() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME,
                "title: 'asd'\ndescription: ''",
                |path| TemplateConfig::load_from(path),
            );
        }
    }

    mod variables {
        use super::*;
        use moon_config2::{
            TemplateVariable, TemplateVariableEnumSetting, TemplateVariableEnumValue,
            TemplateVariableSetting,
        };

        #[test]
        fn loads_boolean() {
            let config = test_load_config(
                CONFIG_TEMPLATE_FILENAME,
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
                TemplateVariable::Boolean(TemplateVariableSetting {
                    default: false,
                    prompt: Some("prompt".into()),
                    required: Some(true)
                })
            );
        }

        #[test]
        #[should_panic(expected = "invalid type: integer `123`, expected a boolean")]
        fn invalid_boolean() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME,
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
                CONFIG_TEMPLATE_FILENAME,
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
                TemplateVariable::Number(TemplateVariableSetting {
                    default: 123,
                    prompt: Some("prompt".into()),
                    required: Some(false)
                })
            );
        }

        #[test]
        #[should_panic(expected = "invalid type: boolean `true`, expected usize")]
        fn invalid_number() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME,
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
                CONFIG_TEMPLATE_FILENAME,
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
                TemplateVariable::String(TemplateVariableSetting {
                    default: "abc".into(),
                    prompt: None,
                    required: None
                })
            );
        }

        #[test]
        #[should_panic(expected = "invalid type: integer `123`, expected a string")]
        fn invalid_string() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME,
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
                CONFIG_TEMPLATE_FILENAME,
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
                    default: "a".into(),
                    multiple: None,
                    prompt: "prompt".into(),
                    values: vec![
                        TemplateVariableEnumValue::String("a".into()),
                        TemplateVariableEnumValue::String("b".into()),
                        TemplateVariableEnumValue::Object {
                            label: "C".into(),
                            value: "c".into()
                        }
                    ],
                })
            );
        }

        #[test]
        #[should_panic(expected = "expected a value string or value object with label")]
        fn invalid_string_enum() {
            test_load_config(
                CONFIG_TEMPLATE_FILENAME,
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
    }
}
