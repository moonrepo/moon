use moon_test_utils::{assert_fs::prelude::*, create_temp_dir, get_fixtures_path};
use moon_typescript_lang::tsconfig::*;
use moon_utils::string_vec;
use starbase_utils::json::{self, JsonValue};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[test]
fn preserves_when_saving() {
    let json = "{\n  \"compilerOptions\": {},\n  \"files\": [\n    \"**/*\"\n  ]\n}\n";

    let dir = create_temp_dir();
    let file = dir.child("tsconfig.json");
    file.write_str(json).unwrap();

    let mut package = TsConfigJson::read(dir.path()).unwrap().unwrap();

    // Trigger dirty
    package.dirty.push("unknown".into());

    package.save().unwrap();

    assert_eq!(std::fs::read_to_string(file.path()).unwrap(), json);
}

#[test]
fn serializes_special_fields() {
    let actual = TsConfigJson {
        compiler_options: Some(CompilerOptions {
            module: Some(Module::EsNext),
            module_resolution: Some(ModuleResolution::Node12),
            jsx: Some(Jsx::ReactJsxdev),
            target: Some(Target::Es6),
            lib: Some(string_vec![
                "dom",
                "es2015.generator",
                "es2016.array.include",
                "es2017.sharedmemory",
                "es2018.intl",
                "es2019.symbol",
                "es2020.symbol.wellknown",
                "es2021.weakref",
            ]),
            ..CompilerOptions::default()
        }),
        ..TsConfigJson::default()
    };

    let expected = serde_json::json!({
        "compilerOptions": {
            "jsx": "react-jsxdev",
            "lib": [
                "dom",
                "es2015.generator",
                "es2016.array.include",
                "es2017.sharedmemory",
                "es2018.intl",
                "es2019.symbol",
                "es2020.symbol.wellknown",
                "es2021.weakref",
            ],
            "module": "esnext",
            "moduleResolution": "node12",
            "target": "es6",
        },
    });

    assert_eq!(
        serde_json::to_string(&actual).unwrap(),
        serde_json::to_string(&expected).unwrap(),
    );
}

#[test]
fn deserializes_special_fields() {
    let actual = serde_json::json!({
        "compilerOptions": {
            "jsx": "react-native",
            "lib": [
                "dom",
                "es2015.collection",
                "es2016",
                "es2017.typedarrays",
                "es2018.promise",
                "es2019.string",
                "es2020",
                "es2021.weakref",
            ],
            "module": "es2015",
            "moduleResolution": "classic",
            "target": "esnext",
        },
    });

    let expected = TsConfigJson {
        compiler_options: Some(CompilerOptions {
            jsx: Some(Jsx::ReactNative),
            lib: Some(string_vec![
                "dom",
                "es2015.collection",
                "es2016",
                "es2017.typedarrays",
                "es2018.promise",
                "es2019.string",
                "es2020",
                "es2021.weakref",
            ]),
            module: Some(Module::Es2015),
            module_resolution: Some(ModuleResolution::Classic),
            target: Some(Target::EsNext),
            ..CompilerOptions::default()
        }),
        ..TsConfigJson::default()
    };

    let actual_typed: TsConfigJson = serde_json::from_value(actual).unwrap();

    assert_eq!(actual_typed, expected);
}

#[test]
fn merge_two_configs() {
    let json_1 = r#"{"compilerOptions": {"jsx": "react", "noEmit": true}}"#;
    let json_2 = r#"{"compilerOptions": {"jsx": "preserve", "removeComments": true}}"#;

    let value1: JsonValue = serde_json::from_str(json_1).unwrap();
    let value2: JsonValue = serde_json::from_str(json_2).unwrap();

    let new_value = json::merge(&value1, &value2);
    let config: TsConfigJson = serde_json::from_value(new_value).unwrap();

    assert_eq!(
        config.clone().compiler_options.unwrap().jsx,
        Some(Jsx::Preserve)
    );
    assert_eq!(config.clone().compiler_options.unwrap().no_emit, Some(true));
    assert_eq!(config.compiler_options.unwrap().remove_comments, Some(true));
}

#[test]
fn parse_basic_file() {
    let path = get_fixtures_path("base/tsconfig-json");
    let config = TsConfigJson::read_with_name(path, "tsconfig.default.json")
        .unwrap()
        .unwrap();

    assert_eq!(
        config.compiler_options.clone().unwrap().target,
        Some(Target::Es5)
    );
    assert_eq!(
        config.compiler_options.clone().unwrap().module,
        Some(Module::CommonJs)
    );
    assert_eq!(config.compiler_options.unwrap().strict, Some(true));
}

#[test]
fn parse_inheriting_file() {
    let path = get_fixtures_path("base/tsconfig-json/tsconfig.inherits.json");
    let config = TsConfigJson::load_with_extends(path).unwrap();

    assert_eq!(
        config
            .compiler_options
            .clone()
            .unwrap()
            .use_define_for_class_fields,
        Some(false)
    );

    assert_eq!(
        config.compiler_options.clone().unwrap().declaration,
        Some(true)
    );

    assert_eq!(
        config.compiler_options.unwrap().trace_resolution,
        Some(false)
    );
}

#[test]
fn parse_inheritance_chain() {
    let path = get_fixtures_path("base/tsconfig-json/a/tsconfig.json");
    let config = TsConfigJson::load_with_extends(path).unwrap();

    assert_eq!(
        config
            .compiler_options
            .clone()
            .unwrap()
            .use_define_for_class_fields,
        Some(false)
    );

    assert_eq!(
        config.compiler_options.clone().unwrap().declaration,
        Some(true)
    );

    assert_eq!(
        config.compiler_options.clone().unwrap().trace_resolution,
        Some(false)
    );

    assert_eq!(config.compiler_options.unwrap().jsx, Some(Jsx::ReactNative));
}

#[test]
fn parse_multi_inheritance_chain() {
    let path = get_fixtures_path("base/tsconfig-json/tsconfig.multi-inherits.json");
    let config = TsConfigJson::load_with_extends(path).unwrap();

    let options = config.compiler_options.as_ref().unwrap();

    assert_eq!(options.declaration, Some(true));
    assert_eq!(options.module_resolution, Some(ModuleResolution::Bundler));
    assert_eq!(options.module, Some(Module::EsNext));
    assert_eq!(options.jsx, Some(Jsx::Preserve));
    assert_eq!(options.trace_resolution, Some(false));
}

mod add_project_ref {
    use super::*;

    #[test]
    fn adds_if_not_set() {
        let mut tsc = TsConfigJson {
            path: PathBuf::from("/base/tsconfig.json"),
            ..TsConfigJson::default()
        };

        assert_eq!(tsc.references, None);

        assert!(tsc
            .add_project_ref(PathBuf::from("/sibling"), "tsconfig.json")
            .unwrap());

        assert_eq!(
            tsc.references.unwrap(),
            vec![Reference {
                path: "../sibling".to_owned(),
                prepend: None,
            }]
        );
    }

    #[test]
    fn doesnt_add_if_set() {
        let mut tsc = TsConfigJson {
            references: Some(vec![Reference {
                path: "../sibling".to_owned(),
                prepend: None,
            }]),
            path: PathBuf::from("/base/tsconfig.json"),
            ..TsConfigJson::default()
        };

        assert!(!tsc
            .add_project_ref(PathBuf::from("/sibling"), "tsconfig.json")
            .unwrap());

        assert_eq!(
            tsc.references.unwrap(),
            vec![Reference {
                path: "../sibling".to_owned(),
                prepend: None,
            }]
        );
    }

    #[test]
    fn includes_custom_config_name() {
        let mut tsc = TsConfigJson {
            path: PathBuf::from("/base/tsconfig.json"),
            ..TsConfigJson::default()
        };

        assert_eq!(tsc.references, None);

        assert!(tsc
            .add_project_ref(PathBuf::from("/sibling"), "tsconfig.ref.json")
            .unwrap());

        assert_eq!(
            tsc.references.unwrap(),
            vec![Reference {
                path: "../sibling/tsconfig.ref.json".to_owned(),
                prepend: None,
            }]
        );
    }

    #[cfg(windows)]
    #[test]
    fn forces_forward_slash() {
        let mut tsc = TsConfigJson {
            path: PathBuf::from("C:\\base\\dir\\tsconfig.json"),
            ..TsConfigJson::default()
        };

        assert_eq!(tsc.references, None);

        assert!(tsc
            .add_project_ref(PathBuf::from("C:\\base\\sibling"), "tsconfig.json")
            .unwrap());

        assert_eq!(
            tsc.references.unwrap(),
            vec![Reference {
                path: "../sibling".to_owned(),
                prepend: None,
            }]
        );
    }

    #[test]
    fn appends_and_sorts_list() {
        let mut tsc = TsConfigJson {
            references: Some(vec![Reference {
                path: "../sister".to_owned(),
                prepend: None,
            }]),
            path: PathBuf::from("/base/tsconfig.json"),
            ..TsConfigJson::default()
        };

        assert!(tsc
            .add_project_ref(PathBuf::from("/brother"), "tsconfig.json")
            .unwrap());

        assert_eq!(
            tsc.references.unwrap(),
            vec![
                Reference {
                    path: "../brother".to_owned(),
                    prepend: None,
                },
                Reference {
                    path: "../sister".to_owned(),
                    prepend: None,
                }
            ]
        );
    }
}

mod update_compiler_options {
    use super::*;

    #[test]
    fn creates_if_none_and_returns_true() {
        let mut tsc = TsConfigJson::default();

        let updated = tsc.update_compiler_options(|options| {
            options.out_dir = Some("./test".into());
            true
        });

        assert!(updated);
        assert_eq!(
            tsc.compiler_options
                .as_ref()
                .unwrap()
                .out_dir
                .as_ref()
                .unwrap(),
            "./test"
        )
    }

    #[test]
    fn doesnt_create_if_none_and_returns_false() {
        let mut tsc = TsConfigJson::default();

        let updated = tsc.update_compiler_options(|options| {
            options.out_dir = Some("./test".into());
            false
        });

        assert!(!updated);
        assert_eq!(tsc.compiler_options, None)
    }

    #[test]
    fn can_update_existing() {
        let mut tsc = TsConfigJson {
            compiler_options: Some(CompilerOptions {
                out_dir: Some("./old".into()),
                ..CompilerOptions::default()
            }),
            ..TsConfigJson::default()
        };

        let updated = tsc.update_compiler_options(|options| {
            options.out_dir = Some("./new".into());
            true
        });

        assert!(updated);
        assert_eq!(
            tsc.compiler_options
                .as_ref()
                .unwrap()
                .out_dir
                .as_ref()
                .unwrap(),
            "./new"
        )
    }

    mod paths {
        use super::*;

        #[test]
        fn sets_if_none() {
            let mut opts = CompilerOptions::default();

            let updated = opts.update_paths(BTreeMap::from_iter([(
                "alias".into(),
                string_vec!["index.ts"],
            )]));

            assert!(updated);
            assert_eq!(
                *opts.paths.as_ref().unwrap().get("alias").unwrap(),
                string_vec!["index.ts"]
            );
        }

        #[test]
        fn sets_multiple() {
            let mut opts = CompilerOptions::default();

            let updated = opts.update_paths(BTreeMap::from_iter([
                ("one".into(), string_vec!["one.ts"]),
                ("two".into(), string_vec!["two.ts"]),
                ("three".into(), string_vec!["three.ts"]),
            ]));

            assert!(updated);
            assert_eq!(opts.paths.as_ref().unwrap().len(), 3);
        }

        #[test]
        fn overrides_existing_value() {
            let mut opts = CompilerOptions {
                paths: Some(BTreeMap::from_iter([(
                    "alias".into(),
                    string_vec!["old.ts"],
                )])),
                ..CompilerOptions::default()
            };

            let updated = opts.update_paths(BTreeMap::from_iter([(
                "alias".into(),
                string_vec!["new.ts"],
            )]));

            assert!(updated);
            assert_eq!(
                *opts.paths.as_ref().unwrap().get("alias").unwrap(),
                string_vec!["new.ts"]
            );
        }

        #[test]
        fn doesnt_overrides_same_value() {
            let mut opts = CompilerOptions {
                paths: Some(BTreeMap::from_iter([(
                    "alias".into(),
                    string_vec!["./src", "./other"],
                )])),
                ..CompilerOptions::default()
            };

            let updated = opts.update_paths(BTreeMap::from_iter([(
                "alias".into(),
                string_vec!["./src", "./other"],
            )]));

            assert!(!updated);

            let updated = opts.update_paths(BTreeMap::from_iter([(
                "alias".into(),
                string_vec!["./other", "./src"],
            )]));

            assert!(!updated);
        }
    }
}
