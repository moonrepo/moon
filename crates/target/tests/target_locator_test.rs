use moon_common::Id;
use moon_target::*;

mod target_locator {
    use super::*;

    mod glob {
        use super::*;

        #[test]
        fn all_scope() {
            assert_eq!(
                TargetLocator::parse(":build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from(":build-*"),
                    scope: Some(TargetScope::All),
                    scope_glob: None,
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("*:build").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("*:build"),
                    scope: Some(TargetScope::All),
                    scope_glob: None,
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
                    scope: Some(TargetScope::Deps),
                    scope_glob: None,
                    task_glob: String::from("build-*"),
                }
            );
        }

        #[test]
        fn self_scope() {
            assert_eq!(
                TargetLocator::parse("~:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("~:build-*"),
                    scope: Some(TargetScope::OwnSelf),
                    scope_glob: None,
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
                    scope: None,
                    scope_glob: Some(String::from("#tag")),
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("#tag-*:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("#tag-*:build-*"),
                    scope: None,
                    scope_glob: Some(String::from("#tag-*")),
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
                    scope: None,
                    scope_glob: Some(String::from("project")),
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("proj-*:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("proj-*:build-*"),
                    scope: None,
                    scope_glob: Some(String::from("proj-*")),
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
                    scope: None,
                    scope_glob: Some(String::from("a/b/**/*")),
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("a/.../b:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("a/.../b:build-*"),
                    scope: None,
                    scope_glob: Some(String::from("a/**/*/b")),
                    task_glob: String::from("build-*"),
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
    }
}
