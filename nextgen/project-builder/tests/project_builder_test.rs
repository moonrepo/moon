use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::{
    DependencyConfig, DependencyScope, DependencySource, InheritedTasksManager, LanguageType,
    PlatformType, TaskCommandArgs, TaskConfig,
};
use moon_file_group::FileGroup;
use moon_platform_detector::detect_project_language;
use moon_project::Project;
use moon_project_builder::ProjectBuilder;
use rustc_hash::FxHashMap;
use starbase_sandbox::create_sandbox;
use std::path::Path;

fn build_project(id: &str, root: &Path) -> Project {
    let mut builder = ProjectBuilder::new(id, id, root).unwrap();
    let manager = InheritedTasksManager::load(root, root.join("global")).unwrap();

    // Use JavaScript so we inherit the correct tasks
    builder.detect_language(|_| LanguageType::JavaScript);

    builder.load_local_config().unwrap();

    builder.inherit_global_config(&manager).unwrap();

    builder.build().unwrap()
}

fn build_lang_project(id: &str) -> Project {
    let sandbox = create_sandbox("langs");

    let mut builder = ProjectBuilder::new(id, id, sandbox.path()).unwrap();

    builder.detect_language(detect_project_language);

    builder.load_local_config().unwrap();

    builder.build().unwrap()
}

mod project_builder {
    use super::*;

    #[test]
    #[should_panic(expected = "No project exists at path qux.")]
    fn errors_missing_source() {
        let sandbox = create_sandbox("builder");

        ProjectBuilder::new("qux", "qux", sandbox.path()).unwrap();
    }

    #[test]
    fn sets_common_fields() {
        let sandbox = create_sandbox("builder");

        let mut builder = ProjectBuilder::new("baz", "baz", sandbox.path()).unwrap();
        builder.load_local_config().unwrap();

        let project = builder.build().unwrap();

        assert_eq!(project.id, Id::raw("baz"));
        assert_eq!(project.source, WorkspaceRelativePathBuf::from("baz"));
        assert_eq!(project.root, sandbox.path().join("baz"));
    }

    #[test]
    fn builds_depends_on() {
        let sandbox = create_sandbox("builder");

        let mut builder = ProjectBuilder::new("baz", "baz", sandbox.path()).unwrap();
        builder.load_local_config().unwrap();

        let project = builder.build().unwrap();

        assert_eq!(
            project.dependencies.into_values().collect::<Vec<_>>(),
            vec![
                DependencyConfig {
                    id: "bar".into(),
                    source: Some(DependencySource::Explicit),
                    ..Default::default()
                },
                DependencyConfig {
                    id: "foo".into(),
                    source: Some(DependencySource::Explicit),
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
        let a = build_project("foo", sandbox.path());
        let b = build_project("bar", sandbox.path());
        let c = build_project("baz", sandbox.path());

        assert_eq!(a.tasks.len(), 4);
        assert_eq!(b.tasks.len(), 3);
        assert_eq!(c.tasks.len(), 5);
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

            let mut builder = ProjectBuilder::new("bar", "bar", sandbox.path()).unwrap();
            builder.load_local_config().unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.language, LanguageType::Rust);
        }

        #[test]
        fn detects_from_env() {
            let sandbox = create_sandbox("builder");

            let mut builder = ProjectBuilder::new("foo", "foo", sandbox.path()).unwrap();
            builder.detect_language(|_| LanguageType::TypeScript);
            builder.load_local_config().unwrap();

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

            let mut builder = ProjectBuilder::new("baz", "baz", sandbox.path()).unwrap();
            builder.load_local_config().unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.platform, PlatformType::Node);
        }

        #[test]
        fn infers_from_config_lang() {
            let sandbox = create_sandbox("builder");

            let mut builder = ProjectBuilder::new("bar", "bar", sandbox.path()).unwrap();
            builder.load_local_config().unwrap();

            let project = builder.build().unwrap();

            assert_eq!(project.platform, PlatformType::Rust);
        }

        #[test]
        fn infers_from_detected_lang() {
            let sandbox = create_sandbox("builder");

            let mut builder = ProjectBuilder::new("foo", "foo", sandbox.path()).unwrap();
            builder.detect_language(|_| LanguageType::TypeScript);
            builder.load_local_config().unwrap();

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

    mod graph_extending {
        use super::*;

        #[test]
        fn inherits_dep() {
            let sandbox = create_sandbox("builder");

            let mut builder = ProjectBuilder::new("bar", "bar", sandbox.path()).unwrap();
            builder.load_local_config().unwrap();

            builder.extend_with_dependency(DependencyConfig {
                id: "foo".into(),
                scope: DependencyScope::Development,
                ..DependencyConfig::default()
            });

            let project = builder.build().unwrap();

            assert_eq!(
                project.dependencies.into_values().collect::<Vec<_>>(),
                vec![DependencyConfig {
                    id: "foo".into(),
                    scope: DependencyScope::Development,
                    source: Some(DependencySource::Implicit),
                    ..DependencyConfig::default()
                }]
            );
        }

        #[test]
        fn doesnt_override_dep_of_same_id() {
            let sandbox = create_sandbox("builder");

            let mut builder = ProjectBuilder::new("baz", "baz", sandbox.path()).unwrap();
            builder.load_local_config().unwrap();

            builder.extend_with_dependency(DependencyConfig {
                id: "foo".into(),
                scope: DependencyScope::Peer,
                ..DependencyConfig::default()
            });

            let project = builder.build().unwrap();

            assert!(project.dependencies.contains_key("foo"));
            assert_eq!(
                project.dependencies.get("foo").unwrap().scope,
                DependencyScope::Development
            );
            assert_eq!(
                project.dependencies.get("foo").unwrap().source,
                Some(DependencySource::Explicit)
            );
        }

        #[test]
        fn inherits_task() {
            let sandbox = create_sandbox("builder");

            let mut builder = ProjectBuilder::new("bar", "bar", sandbox.path()).unwrap();
            builder.load_local_config().unwrap();

            builder.extend_with_task(
                Id::raw("task"),
                TaskConfig {
                    ..TaskConfig::default()
                },
            );

            let project = builder.build().unwrap();

            assert!(project.tasks.contains_key("task"));
        }

        #[test]
        fn doesnt_override_task_of_same_id() {
            let sandbox = create_sandbox("builder");

            let mut builder = ProjectBuilder::new("baz", "baz", sandbox.path()).unwrap();
            builder.load_local_config().unwrap();

            builder.extend_with_task(
                Id::raw("baz"),
                TaskConfig {
                    command: TaskCommandArgs::String("new-command-name".into()),
                    ..TaskConfig::default()
                },
            );

            let project = builder.build().unwrap();

            assert!(project.tasks.contains_key("baz"));
            assert_eq!(project.tasks.get("baz").unwrap().command, "baz");
        }
    }
}
