use moon_common::Id;
use moon_target::*;

mod target_locator {
    use super::*;

    mod project_glob {
        use super::*;

        #[test]
        fn all_scope() {
            assert_eq!(
                TargetLocator::parse(":build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from(":build-*"),
                    project: Some(TargetProjectScope::All),
                    project_glob: None,
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("*:build").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("*:build"),
                    project: Some(TargetProjectScope::All),
                    project_glob: None,
                    task_glob: String::from("build"),
                }
            );
        }

        #[test]
        fn deps_scope() {
            assert_eq!(
                TargetLocator::parse("^:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("^:build-*"),
                    project: Some(TargetProjectScope::Deps),
                    project_glob: None,
                    task_glob: String::from("build-*"),
                }
            );
        }

        #[test]
        fn deps_of_scope() {
            assert_eq!(
                TargetLocator::parse("^build:lint-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("^build:lint-*"),
                    project: Some(TargetProjectScope::DepsOf(TargetDependencyScope::Build)),
                    project_glob: None,
                    task_glob: String::from("lint-*"),
                }
            );
        }

        #[test]
        fn self_scope() {
            assert_eq!(
                TargetLocator::parse("~:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("~:build-*"),
                    project: Some(TargetProjectScope::OwnSelf),
                    project_glob: None,
                    task_glob: String::from("build-*"),
                }
            );
        }

        #[test]
        fn tag_scope() {
            assert_eq!(
                TargetLocator::parse("#tag:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("#tag:build-*"),
                    project: None,
                    project_glob: Some(String::from("#tag")),
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("#tag-*:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("#tag-*:build-*"),
                    project: None,
                    project_glob: Some(String::from("#tag-*")),
                    task_glob: String::from("build-*"),
                }
            );
        }

        #[test]
        fn project_scope() {
            assert_eq!(
                TargetLocator::parse("project:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("project:build-*"),
                    project: None,
                    project_glob: Some(String::from("project")),
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("proj-*:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("proj-*:build-*"),
                    project: None,
                    project_glob: Some(String::from("proj-*")),
                    task_glob: String::from("build-*"),
                }
            );
        }

        #[test]
        fn project_scope_with_bazel_spread() {
            assert_eq!(
                TargetLocator::parse("a/b/...:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("a/b/...:build-*"),
                    project: None,
                    project_glob: Some(String::from("a/b/**/*")),
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("a/.../b:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("a/.../b:build-*"),
                    project: None,
                    project_glob: Some(String::from("a/**/*/b")),
                    task_glob: String::from("build-*"),
                }
            );
        }

        #[test]
        fn task_tag_glob_with_all_scope() {
            assert_eq!(
                TargetLocator::parse(":#test-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from(":#test-*"),
                    project: Some(TargetProjectScope::All),
                    project_glob: None,
                    task_glob: String::from("#test-*"),
                }
            );
        }

        #[test]
        fn task_tag_wildcard() {
            assert_eq!(
                TargetLocator::parse(":#*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from(":#*"),
                    project: Some(TargetProjectScope::All),
                    project_glob: None,
                    task_glob: String::from("#*"),
                }
            );
        }

        #[test]
        fn task_tag_glob_with_self_scope() {
            assert_eq!(
                TargetLocator::parse("~:#test-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("~:#test-*"),
                    project: Some(TargetProjectScope::OwnSelf),
                    project_glob: None,
                    task_glob: String::from("#test-*"),
                }
            );
        }

        #[test]
        fn task_tag_glob_with_deps_scope() {
            assert_eq!(
                TargetLocator::parse("^:#test-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("^:#test-*"),
                    project: Some(TargetProjectScope::Deps),
                    project_glob: None,
                    task_glob: String::from("#test-*"),
                }
            );
        }

        #[test]
        fn task_tag_glob_with_deps_of_scope() {
            assert_eq!(
                TargetLocator::parse("^build:#test-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("^build:#test-*"),
                    project: Some(TargetProjectScope::DepsOf(TargetDependencyScope::Build)),
                    project_glob: None,
                    task_glob: String::from("#test-*"),
                }
            );
        }

        #[test]
        fn task_tag_glob_with_project_scope() {
            assert_eq!(
                TargetLocator::parse("project:#test-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("project:#test-*"),
                    project: None,
                    project_glob: Some(String::from("project")),
                    task_glob: String::from("#test-*"),
                }
            );
        }

        #[test]
        fn task_tag_glob_with_project_tag_scope() {
            // Both the project and task portions are tag globs.
            assert_eq!(
                TargetLocator::parse("#ui-*:#test-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("#ui-*:#test-*"),
                    project: None,
                    project_glob: Some(String::from("#ui-*")),
                    task_glob: String::from("#test-*"),
                }
            );
        }
    }

    mod target {
        use super::*;

        #[test]
        #[should_panic(expected = "Invalid target $:build")]
        fn errors_invalid() {
            TargetLocator::parse("$:build").unwrap();
        }

        #[test]
        fn all_scope() {
            assert_eq!(
                TargetLocator::parse(":build").unwrap(),
                TargetLocator::Qualified(Target::parse(":build").unwrap())
            );
        }

        #[test]
        fn deps_scope() {
            assert_eq!(
                TargetLocator::parse("^:build").unwrap(),
                TargetLocator::Qualified(Target::parse("^:build").unwrap())
            );
        }

        #[test]
        fn deps_of_scope() {
            assert_eq!(
                TargetLocator::parse("^build:lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^build:lint").unwrap())
            );
            assert_eq!(
                TargetLocator::parse("^development:lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^development:lint").unwrap())
            );
            assert_eq!(
                TargetLocator::parse("^dev:lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^dev:lint").unwrap())
            );
            assert_eq!(
                TargetLocator::parse("^peer:lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^peer:lint").unwrap())
            );
            assert_eq!(
                TargetLocator::parse("^production:lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^production:lint").unwrap())
            );
            assert_eq!(
                TargetLocator::parse("^prod:lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^prod:lint").unwrap())
            );
        }

        #[test]
        fn self_scope() {
            assert_eq!(
                TargetLocator::parse("~:build").unwrap(),
                TargetLocator::Qualified(Target::parse("~:build").unwrap())
            );
        }

        #[test]
        fn tag_scope() {
            assert_eq!(
                TargetLocator::parse("#tag:build").unwrap(),
                TargetLocator::Qualified(Target::parse("#tag:build").unwrap())
            );
        }

        #[test]
        fn project_scope() {
            assert_eq!(
                TargetLocator::parse("project:build").unwrap(),
                TargetLocator::Qualified(Target::parse("project:build").unwrap())
            );
        }

        #[test]
        fn task_tag_scope() {
            assert_eq!(
                TargetLocator::parse("project:#lint").unwrap(),
                TargetLocator::Qualified(Target::parse("project:#lint").unwrap())
            );
        }

        #[test]
        fn task_tag_with_all_scope() {
            assert_eq!(
                TargetLocator::parse(":#lint").unwrap(),
                TargetLocator::Qualified(Target::parse(":#lint").unwrap())
            );
        }

        #[test]
        fn task_tag_with_self_scope() {
            assert_eq!(
                TargetLocator::parse("~:#lint").unwrap(),
                TargetLocator::Qualified(Target::parse("~:#lint").unwrap())
            );
        }

        #[test]
        fn task_tag_with_deps_scope() {
            assert_eq!(
                TargetLocator::parse("^:#lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^:#lint").unwrap())
            );
        }

        #[test]
        fn task_tag_with_deps_of_scope() {
            assert_eq!(
                TargetLocator::parse("^build:#lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^build:#lint").unwrap())
            );
            assert_eq!(
                TargetLocator::parse("^development:#lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^development:#lint").unwrap())
            );
            assert_eq!(
                TargetLocator::parse("^peer:#lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^peer:#lint").unwrap())
            );
            assert_eq!(
                TargetLocator::parse("^production:#lint").unwrap(),
                TargetLocator::Qualified(Target::parse("^production:#lint").unwrap())
            );
        }

        #[test]
        fn task_tag_with_project_tag_scope() {
            assert_eq!(
                TargetLocator::parse("#ui:#lint").unwrap(),
                TargetLocator::Qualified(Target::parse("#ui:#lint").unwrap())
            );
        }

        #[test]
        fn task_tag_with_node_package() {
            assert_eq!(
                TargetLocator::parse("@scope/foo:#lint").unwrap(),
                TargetLocator::Qualified(Target::parse("@scope/foo:#lint").unwrap())
            );
        }

        #[test]
        #[should_panic(expected = "Invalid target project:#bad$tag")]
        fn errors_on_invalid_task_tag_chars() {
            TargetLocator::parse("project:#bad$tag").unwrap();
        }
    }

    mod default_project {
        use super::*;

        #[test]
        fn returns_task() {
            assert_eq!(
                TargetLocator::parse("build").unwrap(),
                TargetLocator::DefaultProject(Id::raw("build"))
            );
        }

        #[test]
        #[should_panic]
        fn errors_on_task_tag_without_colon() {
            TargetLocator::parse("#lint").unwrap();
        }
    }
}
