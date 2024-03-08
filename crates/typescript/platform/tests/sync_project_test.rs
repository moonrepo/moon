use moon_common::Id;
use moon_config::TypeScriptConfig;
use moon_project::Project;
use moon_test_utils::{create_sandbox, create_sandbox_with_config, get_node_fixture_configs};
use moon_typescript_lang::tsconfig::*;
use moon_typescript_lang::TsConfigJsonCache;
use moon_typescript_platform::TypeScriptSyncer;
use rustc_hash::FxHashSet;
use std::path::Path;

mod missing_tsconfig {
    use super::*;

    #[test]
    fn creates_tsconfig() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();
        let sandbox = create_sandbox_with_config(
            "node",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let project = Project {
            id: Id::raw("deps-a"),
            root: sandbox.path().join("deps-a"),
            ..Project::default()
        };

        let config = TypeScriptConfig::default();

        let tsconfig_path = project.root.join("tsconfig.json");

        assert!(!tsconfig_path.exists());

        TypeScriptSyncer::new(&project, &config, sandbox.path())
            .create_missing_tsconfig()
            .unwrap();

        assert!(tsconfig_path.exists());

        let tsconfig = TsConfigJsonCache::read(tsconfig_path).unwrap().unwrap();

        assert_eq!(
            tsconfig.data.extends,
            Some(ExtendsField::Single("../tsconfig.options.json".to_owned()))
        );
        assert_eq!(
            tsconfig.data.include,
            Some(vec![PathOrGlob::Glob("**/*".into())])
        );
    }

    #[test]
    fn creates_tsconfig_with_custom_settings() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();
        let sandbox = create_sandbox_with_config(
            "node",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let project = Project {
            id: Id::raw("deps-a"),
            root: sandbox.path().join("deps-a"),
            ..Project::default()
        };

        let config = TypeScriptConfig {
            project_config_file_name: "tsconfig.ref.json".into(),
            root_options_config_file_name: "tsconfig.base.json".into(),
            ..TypeScriptConfig::default()
        };

        let tsconfig_path = project.root.join("tsconfig.ref.json");

        assert!(!tsconfig_path.exists());

        TypeScriptSyncer::new(&project, &config, sandbox.path())
            .create_missing_tsconfig()
            .unwrap();

        assert!(tsconfig_path.exists());

        let tsconfig = TsConfigJsonCache::read_with_name(&project.root, "tsconfig.ref.json")
            .unwrap()
            .unwrap();

        assert_eq!(
            tsconfig.data.extends,
            Some(ExtendsField::Single("../tsconfig.base.json".to_owned()))
        );
        assert_eq!(
            tsconfig.data.include,
            Some(vec![PathOrGlob::Glob("**/*".into())])
        );
    }

    #[test]
    fn doesnt_create_if_a_config_exists() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();
        let sandbox = create_sandbox_with_config(
            "node",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let project = Project {
            id: Id::raw("deps-b"),
            root: sandbox.path().join("deps-b"),
            ..Project::default()
        };

        let config = TypeScriptConfig::default();

        let tsconfig_path = project.root.join("tsconfig.json");

        assert!(tsconfig_path.exists());

        let created = TypeScriptSyncer::new(&project, &config, sandbox.path())
            .create_missing_tsconfig()
            .unwrap();

        assert!(!created);
    }
}

mod sync_root {
    use super::*;

    #[test]
    fn adds_standard() {
        let sandbox = create_sandbox("empty");
        sandbox.create_file("tsconfig.json", "{}");
        sandbox.create_file("project/tsconfig.json", "{}");

        let project = Project {
            id: Id::raw("project"),
            root: sandbox.path().join("project"),
            ..Project::default()
        };

        let config = TypeScriptConfig::default();

        TypeScriptSyncer::new(&project, &config, sandbox.path())
            .sync_as_root_project_reference()
            .unwrap();

        let tsconfig = TsConfigJsonCache::read_with_name(sandbox.path(), "tsconfig.json")
            .unwrap()
            .unwrap();

        assert_eq!(
            tsconfig.data.references.unwrap(),
            vec![ProjectReference {
                path: "project".into(),
                prepend: None
            }]
        );
    }

    #[test]
    fn root_in_sibling_dir() {
        let sandbox = create_sandbox("empty");
        sandbox.create_file("root/tsconfig.json", "{}");
        sandbox.create_file("project/tsconfig.json", "{}");

        let project = Project {
            id: Id::raw("project"),
            root: sandbox.path().join("project"),
            ..Project::default()
        };

        let config = TypeScriptConfig {
            root: "root".into(),
            ..TypeScriptConfig::default()
        };

        TypeScriptSyncer::new(&project, &config, sandbox.path())
            .sync_as_root_project_reference()
            .unwrap();

        let tsconfig = TsConfigJsonCache::read_with_name(sandbox.path(), "root/tsconfig.json")
            .unwrap()
            .unwrap();

        assert_eq!(
            tsconfig.data.references.unwrap(),
            vec![ProjectReference {
                path: "../project".into(),
                prepend: None
            }]
        );
    }

    #[test]
    fn different_names() {
        let sandbox = create_sandbox("empty");
        sandbox.create_file("root/tsconfig.projects.json", "{}");
        sandbox.create_file("a/tsconfig.json", "{}");
        sandbox.create_file("b/tsconfig.build.json", "{}");

        let project = Project {
            id: Id::raw("a"),
            root: sandbox.path().join("a"),
            ..Project::default()
        };

        let config = TypeScriptConfig {
            root_config_file_name: "tsconfig.projects.json".into(),
            root: "root".into(),
            ..TypeScriptConfig::default()
        };

        TypeScriptSyncer::new(&project, &config, sandbox.path())
            .sync_as_root_project_reference()
            .unwrap();

        let project = Project {
            id: Id::raw("b"),
            root: sandbox.path().join("b"),
            ..Project::default()
        };

        let config = TypeScriptConfig {
            project_config_file_name: "tsconfig.build.json".into(),
            root_config_file_name: "tsconfig.projects.json".into(),
            root: "root".into(),
            ..TypeScriptConfig::default()
        };

        TypeScriptSyncer::new(&project, &config, sandbox.path())
            .sync_as_root_project_reference()
            .unwrap();

        let tsconfig =
            TsConfigJsonCache::read_with_name(sandbox.path(), "root/tsconfig.projects.json")
                .unwrap()
                .unwrap();

        assert_eq!(
            tsconfig.data.references.unwrap(),
            vec![
                ProjectReference {
                    path: "../a".into(),
                    prepend: None
                },
                ProjectReference {
                    path: "../b/tsconfig.build.json".into(),
                    prepend: None
                }
            ]
        );
    }

    #[test]
    fn supports_root_project() {
        let sandbox = create_sandbox("empty");
        sandbox.create_file("tsconfig.json", "{}");
        sandbox.create_file("tsconfig.project.json", "{}");

        let project = Project {
            id: Id::raw("root"),
            root: sandbox.path().to_path_buf(),
            ..Project::default()
        };

        let config = TypeScriptConfig {
            project_config_file_name: "tsconfig.project.json".into(),
            ..TypeScriptConfig::default()
        };

        TypeScriptSyncer::new(&project, &config, sandbox.path())
            .sync_as_root_project_reference()
            .unwrap();

        let tsconfig = TsConfigJsonCache::read_with_name(sandbox.path(), "tsconfig.json")
            .unwrap()
            .unwrap();

        assert_eq!(
            tsconfig.data.references.unwrap(),
            vec![ProjectReference {
                path: "tsconfig.project.json".into(),
                prepend: None
            }]
        );
    }

    #[test]
    fn supports_root_project_reversed_config() {
        let sandbox = create_sandbox("empty");
        sandbox.create_file("tsconfig.root.json", "{}");
        sandbox.create_file("tsconfig.json", "{}");

        let project = Project {
            id: Id::raw("root"),
            root: sandbox.path().to_path_buf(),
            ..Project::default()
        };

        let config = TypeScriptConfig {
            root_config_file_name: "tsconfig.root.json".into(),
            ..TypeScriptConfig::default()
        };

        TypeScriptSyncer::new(&project, &config, sandbox.path())
            .sync_as_root_project_reference()
            .unwrap();

        let tsconfig = TsConfigJsonCache::read_with_name(sandbox.path(), "tsconfig.root.json")
            .unwrap()
            .unwrap();

        assert_eq!(
            tsconfig.data.references.unwrap(),
            vec![ProjectReference {
                path: ".".into(),
                prepend: None
            }]
        );
    }

    #[test]
    fn ignores_root_self() {
        let sandbox = create_sandbox("empty");
        sandbox.create_file("tsconfig.json", "{}");

        let project = Project {
            id: Id::raw("root"),
            root: sandbox.path().to_path_buf(),
            ..Project::default()
        };

        let config = TypeScriptConfig::default();

        TypeScriptSyncer::new(&project, &config, sandbox.path())
            .sync_as_root_project_reference()
            .unwrap();

        let tsconfig = TsConfigJsonCache::read_with_name(sandbox.path(), "tsconfig.json")
            .unwrap()
            .unwrap();

        assert_eq!(tsconfig.data.references, None);
    }
}

mod sync_config {
    use super::*;

    mod shared_types {
        use super::*;

        #[test]
        fn adds_when_enabled() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file("packages/project/tsconfig.json", "{}");
            sandbox.create_file("types/index.d.ts", "");

            let project = Project {
                id: Id::raw("project"),
                root: sandbox.path().join("packages/project"),
                ..Project::default()
            };

            let config = TypeScriptConfig {
                include_shared_types: true,
                ..TypeScriptConfig::default()
            };

            TypeScriptSyncer::new(&project, &config, sandbox.path())
                .sync_project_tsconfig(FxHashSet::default())
                .unwrap();

            let tsconfig = TsConfigJsonCache::read_with_name(project.root, "tsconfig.json")
                .unwrap()
                .unwrap();

            assert_eq!(
                tsconfig.data.include.unwrap(),
                vec![PathOrGlob::Glob("../../types/**/*".into())]
            );
        }

        #[test]
        fn doesnt_add_when_enabled_but_already_exists() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "");
            sandbox.create_file(
                "packages/project/tsconfig.json",
                r#"{ "include": ["../../types/**/*"] }"#,
            );
            sandbox.create_file("types/index.d.ts", "");

            let project = Project {
                id: Id::raw("project"),
                root: sandbox.path().join("packages/project"),
                ..Project::default()
            };

            let config = TypeScriptConfig {
                include_shared_types: true,
                ..TypeScriptConfig::default()
            };

            TypeScriptSyncer::new(&project, &config, sandbox.path())
                .sync_project_tsconfig(FxHashSet::default())
                .unwrap();

            let tsconfig = TsConfigJsonCache::read_with_name(project.root, "tsconfig.json")
                .unwrap()
                .unwrap();

            assert_eq!(
                tsconfig.data.include.unwrap(),
                vec![PathOrGlob::Glob("../../types/**/*".into())]
            );
        }

        #[test]
        fn doesnt_add_when_enabled_but_no_types_dir() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file("packages/project/tsconfig.json", "{}");

            let project = Project {
                id: Id::raw("project"),
                root: sandbox.path().join("packages/project"),
                ..Project::default()
            };

            let config = TypeScriptConfig {
                include_shared_types: true,
                ..TypeScriptConfig::default()
            };

            TypeScriptSyncer::new(&project, &config, sandbox.path())
                .sync_project_tsconfig(FxHashSet::default())
                .unwrap();

            let tsconfig = TsConfigJsonCache::read_with_name(project.root, "tsconfig.json")
                .unwrap()
                .unwrap();

            assert_eq!(tsconfig.data.include, None);
        }

        #[test]
        fn doesnt_add_when_disabled() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file("packages/project/tsconfig.json", "{}");

            let project = Project {
                id: Id::raw("project"),
                root: sandbox.path().join("packages/project"),
                ..Project::default()
            };

            let config = TypeScriptConfig {
                include_shared_types: false,
                ..TypeScriptConfig::default()
            };

            TypeScriptSyncer::new(&project, &config, sandbox.path())
                .sync_project_tsconfig(FxHashSet::default())
                .unwrap();

            let tsconfig = TsConfigJsonCache::read_with_name(project.root, "tsconfig.json")
                .unwrap()
                .unwrap();

            assert_eq!(tsconfig.data.include, None);
        }
    }

    mod project_refs {
        use super::*;

        #[test]
        fn adds_when_enabled() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file("packages/a/tsconfig.json", "{}");
            sandbox.create_file("packages/b/tsconfig.json", "{}");
            sandbox.create_file("common/c/tsconfig.json", "{}");

            let project = Project {
                id: Id::raw("project"),
                root: sandbox.path().join("packages/a"),
                ..Project::default()
            };

            let config = TypeScriptConfig {
                sync_project_references: true,
                ..TypeScriptConfig::default()
            };

            TypeScriptSyncer::new(&project, &config, sandbox.path())
                .sync_project_tsconfig(FxHashSet::from_iter([
                    sandbox.path().join("packages/b"),
                    sandbox.path().join("common/c"),
                ]))
                .unwrap();

            let tsconfig = TsConfigJsonCache::read_with_name(project.root, "tsconfig.json")
                .unwrap()
                .unwrap();

            assert_eq!(tsconfig.data.include, None);
            assert_eq!(
                tsconfig.data.references.unwrap(),
                vec![
                    ProjectReference {
                        path: "../../common/c".into(),
                        prepend: None
                    },
                    ProjectReference {
                        path: "../b".into(),
                        prepend: None
                    }
                ]
            );
        }

        #[test]
        fn doesnt_add_when_disabled() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file("packages/a/tsconfig.json", "{}");
            sandbox.create_file("packages/b/tsconfig.json", "{}");
            sandbox.create_file("common/c/tsconfig.json", "{}");

            let project = Project {
                id: Id::raw("project"),
                root: sandbox.path().join("packages/a"),
                ..Project::default()
            };

            let config = TypeScriptConfig {
                sync_project_references: false,
                ..TypeScriptConfig::default()
            };

            TypeScriptSyncer::new(&project, &config, sandbox.path())
                .sync_project_tsconfig(FxHashSet::from_iter([
                    sandbox.path().join("packages/b"),
                    sandbox.path().join("common/c"),
                ]))
                .unwrap();

            let tsconfig = TsConfigJsonCache::read_with_name(project.root, "tsconfig.json")
                .unwrap()
                .unwrap();

            assert_eq!(tsconfig.data.include, None);
            assert_eq!(tsconfig.data.references, None);
        }

        #[test]
        fn includes_sources_when_enabled() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file("packages/a/tsconfig.json", "{}");
            sandbox.create_file("packages/b/tsconfig.json", "{}");
            sandbox.create_file("common/c/tsconfig.json", "{}");

            let project = Project {
                id: Id::raw("project"),
                root: sandbox.path().join("packages/a"),
                ..Project::default()
            };

            let config = TypeScriptConfig {
                include_project_reference_sources: true,
                sync_project_references: true,
                ..TypeScriptConfig::default()
            };

            TypeScriptSyncer::new(&project, &config, sandbox.path())
                .sync_project_tsconfig(FxHashSet::from_iter([
                    sandbox.path().join("packages/b"),
                    sandbox.path().join("common/c"),
                ]))
                .unwrap();

            let tsconfig = TsConfigJsonCache::read_with_name(project.root, "tsconfig.json")
                .unwrap()
                .unwrap();

            assert_eq!(
                tsconfig.data.include.unwrap(),
                vec![
                    PathOrGlob::Glob("../../common/c/**/*".into()),
                    PathOrGlob::Glob("../b/**/*".into())
                ]
            );
            assert_eq!(
                tsconfig.data.references.unwrap(),
                vec![
                    ProjectReference {
                        path: "../../common/c".into(),
                        prepend: None
                    },
                    ProjectReference {
                        path: "../b".into(),
                        prepend: None
                    }
                ]
            );
        }

        #[test]
        fn includes_sources_from_manual_refs() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file("packages/a/tsconfig.json", r#"{ "references": [{ "path": "../b/tsconfig.json" }, { "path": "../../common/c" }] }"#);
            sandbox.create_file("packages/b/tsconfig.json", "{}");
            sandbox.create_file("common/c/tsconfig.json", "{}");

            let project = Project {
                id: Id::raw("project"),
                root: sandbox.path().join("packages/a"),
                ..Project::default()
            };

            let config = TypeScriptConfig {
                include_project_reference_sources: true,
                sync_project_references: true,
                ..TypeScriptConfig::default()
            };

            TypeScriptSyncer::new(&project, &config, sandbox.path())
                .sync_project_tsconfig(FxHashSet::default())
                .unwrap();

            let tsconfig = TsConfigJsonCache::read_with_name(project.root, "tsconfig.json")
                .unwrap()
                .unwrap();

            assert_eq!(
                tsconfig.data.include.unwrap(),
                vec![
                    PathOrGlob::Glob("../../common/c/**/*".into()),
                    PathOrGlob::Glob("../b/**/*".into())
                ]
            );
            assert_eq!(
                tsconfig.data.references.unwrap(),
                vec![
                    ProjectReference {
                        path: "../b/tsconfig.json".into(),
                        prepend: None
                    },
                    ProjectReference {
                        path: "../../common/c".into(),
                        prepend: None
                    },
                ]
            );
        }

        #[test]
        fn doesnt_include_sources_when_sync_disabled() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file("packages/a/tsconfig.json", "{}");
            sandbox.create_file("packages/b/tsconfig.json", "{}");
            sandbox.create_file("common/c/tsconfig.json", "{}");

            let project = Project {
                id: Id::raw("project"),
                root: sandbox.path().join("packages/a"),
                ..Project::default()
            };

            let config = TypeScriptConfig {
                include_project_reference_sources: true,
                sync_project_references: false,
                ..TypeScriptConfig::default()
            };

            TypeScriptSyncer::new(&project, &config, sandbox.path())
                .sync_project_tsconfig(FxHashSet::from_iter([
                    sandbox.path().join("packages/b"),
                    sandbox.path().join("common/c"),
                ]))
                .unwrap();

            let tsconfig = TsConfigJsonCache::read_with_name(project.root, "tsconfig.json")
                .unwrap()
                .unwrap();

            assert_eq!(tsconfig.data.include, None);
            assert_eq!(tsconfig.data.references, None);
        }
    }

    mod paths {
        use super::*;

        fn run_for_a(root: &Path) -> TsConfigJsonCache {
            let project = Project {
                id: Id::raw("project"),
                root: root.join("packages/a"),
                ..Project::default()
            };

            let config = TypeScriptConfig {
                create_missing_config: true,
                sync_project_references: true,
                sync_project_references_to_paths: true,
                ..TypeScriptConfig::default()
            };

            TypeScriptSyncer::new(&project, &config, root)
                .sync_project_tsconfig(FxHashSet::from_iter([
                    root.join("packages/b"),
                    root.join("common/c"),
                ]))
                .unwrap();

            TsConfigJsonCache::read_with_name(project.root, "tsconfig.json")
                .unwrap()
                .unwrap()
        }

        #[test]
        fn adds_wildcards() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file("packages/a/tsconfig.json", "{}");
            sandbox.create_file("packages/a/package.json", r#"{ "name": "a" }"#);
            sandbox.create_file("packages/b/package.json", r#"{ "name": "b" }"#);
            sandbox.create_file("packages/b/src/file.ts", ""); // Not index on purpose
            sandbox.create_file("common/c/package.json", r#"{ "name": "c" }"#);

            let tsconfig = run_for_a(sandbox.path());

            assert_eq!(
                tsconfig.data.compiler_options.unwrap().paths.unwrap(),
                CompilerOptionsPathsMap::from_iter([
                    ("b/*".into(), vec!["../b/src/*".into()]),
                    ("c/*".into(), vec!["../../common/c/*".into()]),
                ])
            );
        }

        #[test]
        fn adds_indexes() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file("packages/a/tsconfig.json", "{}");
            sandbox.create_file("packages/a/package.json", r#"{ "name": "a" }"#);
            sandbox.create_file("packages/b/package.json", r#"{ "name": "b" }"#);
            sandbox.create_file("packages/b/src/index.ts", "");
            sandbox.create_file("common/c/package.json", r#"{ "name": "c" }"#);
            sandbox.create_file("common/c/index.ts", "");

            let tsconfig = run_for_a(sandbox.path());

            assert_eq!(
                tsconfig.data.compiler_options.unwrap().paths.unwrap(),
                CompilerOptionsPathsMap::from_iter([
                    ("b".into(), vec!["../b/src/index.ts".into()]),
                    ("b/*".into(), vec!["../b/src/*".into()]),
                    ("c".into(), vec!["../../common/c/index.ts".into()]),
                    ("c/*".into(), vec!["../../common/c/*".into()]),
                ])
            );
        }

        #[test]
        fn adds_from_manual_refs() {
            let sandbox = create_sandbox("empty");
            sandbox.create_file("tsconfig.json", "{}");
            sandbox.create_file(
                "packages/a/tsconfig.json",
                r#"{ "references": [{ "path": "../d" }] }"#,
            );
            sandbox.create_file("packages/a/package.json", r#"{ "name": "a" }"#);
            sandbox.create_file("packages/b/package.json", r#"{ "name": "b" }"#);
            sandbox.create_file("packages/b/src/file.ts", ""); // Not index on purpose
            sandbox.create_file("common/c/package.json", r#"{ "name": "c" }"#);
            sandbox.create_file("packages/d/package.json", r#"{ "name": "d" }"#);
            sandbox.create_file("packages/d/index.ts", "");

            let tsconfig = run_for_a(sandbox.path());

            assert_eq!(
                tsconfig.data.compiler_options.unwrap().paths.unwrap(),
                CompilerOptionsPathsMap::from_iter([
                    ("b/*".into(), vec!["../b/src/*".into()]),
                    ("c/*".into(), vec!["../../common/c/*".into()]),
                    ("d".into(), vec!["../d/index.ts".into()]),
                    ("d/*".into(), vec!["../d/*".into()]),
                ])
            );
        }
    }
}
