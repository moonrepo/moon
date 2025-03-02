use moon_common::Id;
use moon_config::TypeScriptConfig;
use moon_project::Project;
use moon_test_utils::create_sandbox;
use moon_typescript_lang::TsConfigJsonCache;
use moon_typescript_lang::tsconfig::*;
use moon_typescript_platform::TypeScriptSyncer;
use rustc_hash::FxHashSet;
use std::path::Path;

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
                vec![CompilerPath::from("../../types/**/*")]
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
                vec![CompilerPath::from("../../types/**/*")]
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
                    CompilerPath::from("../../common/c/**/*"),
                    CompilerPath::from("../b/**/*"),
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
                    CompilerPath::from("../../common/c/**/*"),
                    CompilerPath::from("../b/**/*")
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
