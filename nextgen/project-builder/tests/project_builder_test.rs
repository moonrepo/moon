use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::{
    DependencyConfig, DependencyScope, InheritedTasksConfig, InheritedTasksManager, LanguageType,
    PlatformType,
};
use moon_file_group::FileGroup;
use moon_project2::Project;
use moon_project_builder::ProjectBuilder;
use rustc_hash::FxHashMap;
use starbase_sandbox::create_sandbox;
use std::path::Path;

fn load_tasks_into_manager(workspace_root: &Path) -> InheritedTasksManager {
    let mut manager = InheritedTasksManager::default();
    let tasks_path = workspace_root.join("global/tasks.yml");

    manager.add_config(
        &tasks_path,
        InheritedTasksConfig::load_partial(workspace_root, &tasks_path).unwrap(),
    );

    let tasks_dir = tasks_path.parent().unwrap().join("tasks");

    if !tasks_dir.exists() {
        return manager;
    }

    for file in std::fs::read_dir(tasks_dir).unwrap() {
        let file = file.unwrap();

        if file.file_type().unwrap().is_file() {
            manager.add_config(
                &file.path(),
                InheritedTasksConfig::load_partial(workspace_root, &file.path()).unwrap(),
            );
        }
    }

    manager
}

fn build_project(id: &str, root: &Path) -> Project {
    let mut builder = ProjectBuilder::new(id.into(), id.into(), root).unwrap();
    let manager = load_tasks_into_manager(root);

    // Use JavaScript so we inherit the correct tasks
    builder
        .load_local_config(|_| LanguageType::JavaScript)
        .unwrap();

    builder.inherit_global_config(&manager).unwrap();

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
}
