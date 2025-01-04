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
                    project_glob: None,
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("*:build").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("*:build"),
                    scope: Some(TargetScope::All),
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
                    scope: Some(TargetScope::Deps),
                    project_glob: None,
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
                    scope: None,
                    project_glob: Some(String::from("#tag")),
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("#tag-*:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("#tag-*:build-*"),
                    scope: None,
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
                    scope: None,
                    project_glob: Some(String::from("project")),
                    task_glob: String::from("build-*"),
                }
            );

            assert_eq!(
                TargetLocator::parse("proj-*:build-*").unwrap(),
                TargetLocator::GlobMatch {
                    original: String::from("proj-*:build-*"),
                    scope: None,
                    project_glob: Some(String::from("proj-*")),
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

    mod cwd {
        use super::*;

        #[test]
        fn returns_task() {
            assert_eq!(
                TargetLocator::parse("build").unwrap(),
                TargetLocator::TaskFromWorkingDir(Id::raw("build"))
            );
        }
    }
}
