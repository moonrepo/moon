use moon_app::commands::generate::parse_args_into_variables;
use moon_codegen::tera::{Number, Value};
use moon_common::path::standardize_separators;
use moon_config::{
    TemplateVariable, TemplateVariableBoolSetting, TemplateVariableEnumSetting,
    TemplateVariableEnumValue, TemplateVariableNumberSetting, TemplateVariableStringSetting,
};
use moon_test_utils::{
    Sandbox, assert_snapshot, create_sandbox_with_config, predicates::prelude::*,
};
use rustc_hash::FxHashMap;
use std::fs;

fn generate_sandbox() -> Sandbox {
    create_sandbox_with_config("generator", None, None, None)
}

#[test]
fn creates_a_new_template() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate").arg("new-name").arg("--template");
    });

    let output = assert.output();

    assert!(predicate::str::contains("Created a new template new-name at").eval(&output));
    assert!(sandbox.path().join("templates/new-name").exists());

    assert.success();
}

#[test]
fn generates_files_from_template() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate").arg("standard").arg("./test");
    });

    assert_snapshot!(assert.output_standardized());

    assert!(sandbox.path().join("test").exists());
    assert!(sandbox.path().join("test/file.ts").exists());
    assert!(sandbox.path().join("test/folder/nested-file.ts").exists());
    assert!(sandbox.path().join("test/image.jpg").exists());
    assert!(!sandbox.path().join("test/template.yml").exists());
}

#[test]
fn generates_files_into_default_dest() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate").arg("dest");
    });

    assert_snapshot!(assert.output_standardized());

    assert!(sandbox.path().join("apps/foo-bar/file.txt").exists());
}

#[test]
fn generates_files_into_workspace_relative_dest() {
    let sandbox = generate_sandbox();
    sandbox.create_file("sub/dir/file.txt", "");

    sandbox
        .run_moon(|cmd| {
            cmd.arg("generate")
                .arg("dest")
                .arg("/custom/dest")
                .current_dir(sandbox.path().join("sub/dir"));
        })
        .success();

    assert!(sandbox.path().join("custom/dest/file.txt").exists());
}

#[test]
fn generates_files_into_cwd_relative_dest() {
    let sandbox = generate_sandbox();
    sandbox.create_file("sub/dir/file.txt", "");

    sandbox
        .run_moon(|cmd| {
            cmd.arg("generate")
                .arg("dest")
                .arg("custom/dest")
                .current_dir(sandbox.path().join("sub/dir"));
        })
        .success();

    assert!(sandbox.path().join("sub/dir/custom/dest/file.txt").exists());
}

#[test]
fn doesnt_generate_files_when_dryrun() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("standard")
            .arg("./test")
            .arg("--dryRun");
    });

    assert_snapshot!(assert.output_standardized());

    assert!(!sandbox.path().join("test").exists());
    assert!(!sandbox.path().join("test/file.ts").exists());
    assert!(!sandbox.path().join("test/folder/nested-file.ts").exists());
    assert!(!sandbox.path().join("test/template.yml").exists());
}

#[test]
fn overwrites_existing_files_when_forced() {
    let sandbox = generate_sandbox();

    sandbox.run_moon(|cmd| {
        cmd.arg("generate").arg("standard").arg("./test");
    });

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("standard")
            .arg("./test")
            .arg("--force");
    });

    assert_snapshot!(assert.output_standardized());

    assert!(sandbox.path().join("test").exists());
    assert!(sandbox.path().join("test/file.ts").exists());
    assert!(sandbox.path().join("test/folder/nested-file.ts").exists());
    assert!(!sandbox.path().join("test/template.yml").exists());
}

#[test]
fn overwrites_existing_files_when_interpolated_path() {
    let sandbox = generate_sandbox();

    sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults");
    });

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults")
            .arg("--force");
    });

    assert_snapshot!(assert.output_standardized());

    // file-[stringNotEmpty]-[number].txt
    assert!(sandbox.path().join("./test/file-default-0.txt").exists());
}

#[test]
fn renders_and_interpolates_templates() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults");
    });

    assert.success();

    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/expressions.txt")).unwrap());
    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/control.txt")).unwrap());
}

#[test]
fn renders_with_custom_vars_via_args() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults")
            .arg("--")
            .args([
                "--no-boolTrue",
                "--boolFalse",
                "--string=abc",
                "--stringNotEmpty",
                "xyz",
                "--number=123",
                "--numberNotEmpty",
                "456",
                "--enum=c",
                "--multenumNotEmpty",
                "a",
            ]);
    });

    assert.success();

    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/expressions.txt")).unwrap());
    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/control.txt")).unwrap());
}

#[test]
fn cant_overwrite_internal_variables_with_args() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults")
            .arg("--")
            .args(["--internal", "overwritten"]);
    });

    // It errors!
    assert.failure();
}

#[test]
fn handles_raw_files() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate").arg("standard").arg("./test");
    });

    assert.success();

    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/file.txt")).unwrap());
    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/other.txt")).unwrap());
}

#[test]
fn interpolates_destination_path() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults");
    });

    // Verify output paths are correct
    assert_snapshot!(assert.output_standardized());

    // file-[stringNotEmpty]-[number].txt
    assert!(sandbox.path().join("./test/file-default-0.txt").exists());
}

#[test]
fn errors_when_parsing_custom_var_types() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults")
            .arg("--")
            .arg("--number=abc");
    });

    assert_snapshot!(assert.output_standardized());
}

#[test]
fn supports_custom_filters() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults");
    });

    assert.success();

    let content = fs::read_to_string(sandbox.path().join("./test/filters.txt")).unwrap();

    assert_snapshot!(standardize_separators(content));
}

#[test]
fn supports_tera_twig_exts() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("extensions")
            .arg("./test")
            .arg("--defaults");
    });

    assert.success();

    let tera = sandbox.path().join("./test/file.ts");
    let twig = sandbox.path().join("./test/file.tsx");

    assert!(tera.exists());
    assert!(twig.exists());

    assert_eq!(
        fs::read_to_string(tera).unwrap(),
        "export type FooBar = true;\n"
    );
    assert_eq!(
        fs::read_to_string(twig).unwrap(),
        "export type FooBar = true;\n"
    );
}

mod extends {
    use super::*;

    #[test]
    fn generates_files_from_all_templates() {
        let sandbox = generate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("generate").arg("extends").arg("./test");
        });

        assert_snapshot!(assert.output_standardized());

        assert!(sandbox.path().join("test").exists());
        assert!(sandbox.path().join("test/base.txt").exists());
        assert!(sandbox.path().join("test/one.txt").exists());
        assert!(sandbox.path().join("test/two.txt").exists());
        assert!(sandbox.path().join("test/vars.txt").exists());
    }

    #[test]
    fn primary_files_overwrite_extended_files() {
        let sandbox = generate_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("generate").arg("extends").arg("./test");
        });

        assert_eq!(
            fs::read_to_string(sandbox.path().join("test/two.txt")).unwrap(),
            "two overwritten\n"
        );
    }

    #[test]
    fn primary_file_can_use_vars_from_extended() {
        let sandbox = generate_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("generate")
                .arg("extends")
                .arg("./test")
                .arg("--")
                .arg("--one")
                .arg("abc")
                .arg("--two")
                .arg("123");
        });

        assert_snapshot!(fs::read_to_string(sandbox.path().join("test/vars.txt")).unwrap());
    }
}

mod frontmatter {
    use super::*;

    #[test]
    fn changes_dest_path() {
        let sandbox = generate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("generate")
                .arg("frontmatter")
                .arg("./test")
                .arg("--defaults");
        });

        assert.success();

        assert!(!sandbox.path().join("./test/to.txt").exists());
        assert!(sandbox.path().join("./test/to-NEW.txt").exists());
        assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/to-NEW.txt")).unwrap());
    }

    #[test]
    fn force_writes_file() {
        let sandbox = generate_sandbox();

        sandbox.create_file("test/forced.txt", "Original content");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("generate")
                .arg("frontmatter")
                .arg("./test")
                .arg("--defaults");
        });

        assert.success();

        assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/forced.txt")).unwrap());
    }

    #[test]
    fn skips_over_file() {
        let sandbox = generate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("generate")
                .arg("frontmatter")
                .arg("./test")
                .arg("--defaults");
        });

        assert.success();

        assert!(!sandbox.path().join("./test/skipped.txt").exists());
    }

    #[test]
    fn supports_component_vars() {
        let sandbox = generate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("generate")
                .arg("frontmatter")
                .arg("./test")
                .arg("--defaults");
        });

        assert.success();

        assert!(
            sandbox
                .path()
                .join("./test/components/SmallButton.tsx")
                .exists()
        );
        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("./test/components/SmallButton.tsx")).unwrap()
        );
    }
}

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
