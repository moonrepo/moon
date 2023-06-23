use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::{
    DependencyConfig, DependencyScope, InheritedTasksManager, LanguageType, PlatformType,
};
use moon_file_group::FileGroup;
use moon_platform_detector::detect_project_language;
use moon_project2::Project;
use moon_project_builder::ProjectBuilder;
use rustc_hash::FxHashMap;
use starbase_sandbox::create_sandbox;
use std::path::Path;

fn build_project(id: &str, root: &Path) -> Project {
    let mut builder = ProjectBuilder::new(id.into(), id.into(), root).unwrap();
    let manager = InheritedTasksManager::load(root, root.join("global")).unwrap();

    // Use JavaScript so we inherit the correct tasks
    builder
        .load_local_config(|_| LanguageType::JavaScript)
        .unwrap();

    builder.inherit_global_config(&manager).unwrap();

    builder.build().unwrap()
}

fn build_lang_project(id: &str) -> Project {
    let sandbox = create_sandbox("langs");

    let mut builder = ProjectBuilder::new(id.into(), id.into(), sandbox.path()).unwrap();
    builder.load_local_config(detect_project_language).unwrap();

    builder.build().unwrap()
}

mod project_builder {
    use super::*;

    #[test]
    #[should_panic(expected = "MissingProjectAtSource(\"qux\")")]
    fn errors_missing_source() {
        let sandbox = create_sandbox("builder");

        ProjectBuilder::new("qux".into(), "qux".into(), sandbox.path()).unwrap();
    }

    #[test]
    fn sets_common_fields() {
        let sandbox = create_sandbox("builder");

        let mut builder = ProjectBuilder::new("baz".into(), "baz".into(), sandbox.path()).unwrap();
        builder
            .load_local_config(|_| LanguageType::Unknown)
            .unwrap();

        let project = builder.build().unwrap();

        assert_eq!(project.id, Id::raw("baz"));
        assert_eq!(project.source, WorkspaceRelativePathBuf::from("baz"));
        assert_eq!(project.root, sandbox.path().join("baz"));
    }

    #[test]
    fn builds_depends_on() {
        let sandbox = create_sandbox("builder");

        let mut builder = ProjectBuilder::new("baz".into(), "baz".into(), sandbox.path()).unwrap();
        builder
            .load_local_config(|_| LanguageType::Unknown)
            .unwrap();

        let project = builder.build().unwrap();

        assert_eq!(
            project.dependencies.into_values().collect::<Vec<_>>(),
            vec![
                DependencyConfig {
                    id: "bar".into(),
                    ..Default::default()
                },
                DependencyConfig {
                    id: "foo".into(),
                    scope: DependencyScope::Development,
                    ..Default::default()
                }
            ]
        );
    }

    // Tasks are tested heavily in the tasks-builder crate
    #[test]
    fn builds_tasks() {
        let sandbox = create_sandbox("builder");
        let foo = build_project("foo", sandbox.path());
        let bar = build_project("bar", sandbox.path());
        let baz = build_project("baz", sandbox.path());

        assert_eq!(foo.tasks.len(), 0);
        assert_eq!(bar.tasks.len(), 1);
        assert_eq!(baz.tasks.len(), 1);
    }

    mod file_groups {
        use super::*;

        #[test]
        fn inherits_from_global_when_no_local() {
            let sandbox = create_sandbox("builder");
            let project = build_project("foo", sandbox.path());

            assert_eq!(
                project.file_groups,
                FxHashMap::from_iter([
                    (
                        "sources".into(),
                        FileGroup::new_with_source(
                            "sources",
                            [WorkspaceRelativePathBuf::from("foo/node")]
                        )
                        .unwrap()
                    ),
                    (
                        "tests".into(),
                        FileGroup::new_with_source(
                            "tests",
                            [WorkspaceRelativePathBuf::from("foo/global")]
                        )
                        .unwrap()
                    ),
                    (
                        "other".into(),
                        FileGroup::new_with_source(
                            "other",
                            [WorkspaceRelativePathBuf::from("foo/global")]
                        )
                        .unwrap()
                    )
                ])
            );
        }

        #[test]
        fn inherits_from_global_but_local_overrides() {
            let sandbox = create_sandbox("builder");
            let project = build_project("bar", sandbox.path());

            assert_eq!(
                project.file_groups,
                FxHashMap::from_iter([
                    (
                        "sources".into(),
                        FileGroup::new_with_source(
                            "sources",
                            // Not node since the language is rust
                            [WorkspaceRelativePathBuf::from("bar/global")]
                        )
                        .unwrap()
                    ),
                    (
                        "tests".into(),
                        FileGroup::new_with_source(
                            "tests",
                            [WorkspaceRelativePathBuf::from("bar/global")]
                        )
                        .unwrap()
                    ),
                    (
                        "other".into(),
                        FileGroup::new_with_source(
                            "other",
                            [WorkspaceRelativePathBuf::from("bar/bar")]
                        )
                        .unwrap()
                    )
                ])
            );
        }
    }

    mod language_detect {
        use super::*;

        #[test]
        fn inherits_from_config() {
            let sandbox = create_sandbox("builder");

            let mut builder =
                ProjectBuilder::new("bar".into(), "bar".into(), sandbox.path()).unwrap();
            builder
                .load_local_config(|_| LanguageType::Unknown)
                .unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.language, LanguageType::Rust);
        }

        #[test]
        fn detects_from_env() {
            let sandbox = create_sandbox("builder");

            let mut builder =
                ProjectBuilder::new("foo".into(), "foo".into(), sandbox.path()).unwrap();
            builder
                .load_local_config(|_| LanguageType::TypeScript)
                .unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.language, LanguageType::TypeScript);
        }

        #[test]
        fn detects_bash() {
            let project = build_lang_project("bash");

            assert_eq!(project.language, LanguageType::Bash);
            assert_eq!(project.platform, PlatformType::System);
        }

        #[test]
        fn detects_batch() {
            let project = build_lang_project("batch");

            assert_eq!(project.language, LanguageType::Batch);
            assert_eq!(project.platform, PlatformType::System);
        }

        #[test]
        fn detects_deno() {
            let project = build_lang_project("deno");

            assert_eq!(project.language, LanguageType::JavaScript);
            assert_eq!(project.platform, PlatformType::Deno);

            let project = build_lang_project("deno-config");

            assert_eq!(project.language, LanguageType::TypeScript);
            // assert_eq!(project.platform, PlatformType::Deno);
        }

        #[test]
        fn detects_go() {
            let project = build_lang_project("go");

            assert_eq!(project.language, LanguageType::Go);
            assert_eq!(project.platform, PlatformType::System);

            let project = build_lang_project("go-config");

            assert_eq!(project.language, LanguageType::Go);
            assert_eq!(project.platform, PlatformType::System);
        }

        #[test]
        fn detects_js() {
            let project = build_lang_project("js");

            assert_eq!(project.language, LanguageType::JavaScript);
            assert_eq!(project.platform, PlatformType::Node);

            let project = build_lang_project("js-config");

            assert_eq!(project.language, LanguageType::JavaScript);
            assert_eq!(project.platform, PlatformType::Node);
        }

        #[test]
        fn detects_other() {
            let project = build_lang_project("other");

            assert_eq!(project.language, LanguageType::Other("kotlin".into()));
            assert_eq!(project.platform, PlatformType::System);
        }

        #[test]
        fn detects_php() {
            let project = build_lang_project("php");

            assert_eq!(project.language, LanguageType::Php);
            assert_eq!(project.platform, PlatformType::System);

            let project = build_lang_project("php-config");

            assert_eq!(project.language, LanguageType::Php);
            assert_eq!(project.platform, PlatformType::System);
        }

        #[test]
        fn detects_python() {
            let project = build_lang_project("python");

            assert_eq!(project.language, LanguageType::Python);
            assert_eq!(project.platform, PlatformType::System);

            let project = build_lang_project("python-config");

            assert_eq!(project.language, LanguageType::Python);
            assert_eq!(project.platform, PlatformType::System);
        }

        #[test]
        fn detects_ruby() {
            let project = build_lang_project("ruby");

            assert_eq!(project.language, LanguageType::Ruby);
            assert_eq!(project.platform, PlatformType::System);

            let project = build_lang_project("ruby-config");

            assert_eq!(project.language, LanguageType::Ruby);
            assert_eq!(project.platform, PlatformType::System);
        }

        #[test]
        fn detects_rust() {
            let project = build_lang_project("rust");

            assert_eq!(project.language, LanguageType::Rust);
            assert_eq!(project.platform, PlatformType::Rust);

            let project = build_lang_project("rust-config");

            assert_eq!(project.language, LanguageType::Rust);
            assert_eq!(project.platform, PlatformType::Rust);
        }

        #[test]
        fn detects_ts() {
            let project = build_lang_project("ts");

            assert_eq!(project.language, LanguageType::TypeScript);
            assert_eq!(project.platform, PlatformType::Node);

            let project = build_lang_project("ts-config");

            assert_eq!(project.language, LanguageType::TypeScript);
            assert_eq!(project.platform, PlatformType::Node);
        }
    }

    mod platform_detect {
        use super::*;

        #[test]
        fn inherits_from_config() {
            let sandbox = create_sandbox("builder");

            let mut builder =
                ProjectBuilder::new("baz".into(), "baz".into(), sandbox.path()).unwrap();
            builder
                .load_local_config(|_| LanguageType::Unknown)
                .unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.platform, PlatformType::Node);
        }

        #[test]
        fn infers_from_config_lang() {
            let sandbox = create_sandbox("builder");

            let mut builder =
                ProjectBuilder::new("bar".into(), "bar".into(), sandbox.path()).unwrap();
            builder
                .load_local_config(|_| LanguageType::Unknown)
                .unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.platform, PlatformType::Rust);
        }

        #[test]
        fn infers_from_detected_lang() {
            let sandbox = create_sandbox("builder");

            let mut builder =
                ProjectBuilder::new("foo".into(), "foo".into(), sandbox.path()).unwrap();
            builder
                .load_local_config(|_| LanguageType::TypeScript)
                .unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.platform, PlatformType::Node);
        }

        #[test]
        fn fallsback_to_project() {
            let project = build_lang_project("project-platform");

            assert_eq!(
                project.get_task("node-a").unwrap().platform,
                PlatformType::Node
            );

            assert_eq!(
                project.get_task("node-b").unwrap().platform,
                PlatformType::Node
            );

            assert_eq!(
                project.get_task("system").unwrap().platform,
                PlatformType::System
            );
        }
    }
}
