use compact_str::CompactString;
use moon_common::Id;
use moon_target::{DependencyScope, Target, TargetScope};

#[test]
#[should_panic(expected = "Invalid target foo$:build")]
fn errors_on_invalid_chars() {
    Target::parse("foo$:build").unwrap();
}

#[test]
#[should_panic(expected = "Invalid target foo:@build")]
fn errors_on_invalid_task_no_at() {
    Target::parse("foo:@build").unwrap();
}

#[test]
#[should_panic(expected = "Wildcard scope and task not supported.")]
fn errors_on_too_wild() {
    Target::parse(":").unwrap();
}

#[test]
fn format_all_scope() {
    assert_eq!(Target::format(TargetScope::All, "build"), ":build");
}

#[test]
fn format_deps_scope() {
    assert_eq!(Target::format(TargetScope::Deps, "build"), "^:build");
}

#[test]
fn format_deps_of_build_scope() {
    assert_eq!(
        Target::format(TargetScope::DepsOf(DependencyScope::Build), "build"),
        "^build:build"
    );
}

#[test]
fn format_deps_of_development_scope() {
    assert_eq!(
        Target::format(TargetScope::DepsOf(DependencyScope::Development), "build"),
        "^development:build"
    );
}

#[test]
fn format_deps_of_peer_scope() {
    assert_eq!(
        Target::format(TargetScope::DepsOf(DependencyScope::Peer), "build"),
        "^peer:build"
    );
}

#[test]
fn format_deps_of_production_scope() {
    assert_eq!(
        Target::format(TargetScope::DepsOf(DependencyScope::Production), "build"),
        "^production:build"
    );
}

#[test]
fn format_self_scope() {
    assert_eq!(Target::format(TargetScope::OwnSelf, "build"), "~:build");
}

#[test]
fn format_project_scope() {
    assert_eq!(
        Target::format(TargetScope::Project(Id::raw("foo")), "build"),
        "foo:build"
    );
}

#[test]
fn format_tag_scope() {
    assert_eq!(
        Target::format(TargetScope::Tag(Id::raw("foo")), "build"),
        "#foo:build"
    );
}

#[test]
fn format_with_slashes() {
    assert_eq!(
        Target::format(TargetScope::Project(Id::raw("foo/sub")), "build/esm"),
        "foo/sub:build/esm"
    );
}

#[test]
fn format_node_package() {
    assert_eq!(
        Target::format(TargetScope::Project(Id::raw("@scope/foo")), "build"),
        "@scope/foo:build"
    );
}

#[test]
fn parse_ids() {
    assert_eq!(
        Target::parse("foo:build").unwrap(),
        Target {
            id: CompactString::from("foo:build"),
            scope: TargetScope::Project(Id::raw("foo")),
            task_id: Id::raw("build"),
        }
    );
}

#[test]
fn parse_deps_scope() {
    assert_eq!(
        Target::parse("^:build").unwrap(),
        Target {
            id: CompactString::from("^:build"),
            scope: TargetScope::Deps,
            task_id: Id::raw("build"),
        }
    );
}

#[test]
fn parse_deps_of_build_scope() {
    assert_eq!(
        Target::parse("^build:lint").unwrap(),
        Target {
            id: CompactString::from("^build:lint"),
            scope: TargetScope::DepsOf(DependencyScope::Build),
            task_id: Id::raw("lint"),
        }
    );
}

#[test]
fn parse_deps_of_development_scope() {
    assert_eq!(
        Target::parse("^development:lint").unwrap(),
        Target {
            id: CompactString::from("^development:lint"),
            scope: TargetScope::DepsOf(DependencyScope::Development),
            task_id: Id::raw("lint"),
        }
    );
    assert_eq!(
        Target::parse("^dev:lint").unwrap(),
        Target {
            id: CompactString::from("^development:lint"),
            scope: TargetScope::DepsOf(DependencyScope::Development),
            task_id: Id::raw("lint"),
        }
    );
}

#[test]
fn parse_deps_of_peer_scope() {
    assert_eq!(
        Target::parse("^peer:lint").unwrap(),
        Target {
            id: CompactString::from("^peer:lint"),
            scope: TargetScope::DepsOf(DependencyScope::Peer),
            task_id: Id::raw("lint"),
        }
    );
}

#[test]
fn parse_deps_of_production_scope() {
    assert_eq!(
        Target::parse("^production:lint").unwrap(),
        Target {
            id: CompactString::from("^production:lint"),
            scope: TargetScope::DepsOf(DependencyScope::Production),
            task_id: Id::raw("lint"),
        }
    );
    assert_eq!(
        Target::parse("^prod:lint").unwrap(),
        Target {
            id: CompactString::from("^production:lint"),
            scope: TargetScope::DepsOf(DependencyScope::Production),
            task_id: Id::raw("lint"),
        }
    );
}

// #[test]
// fn parse_deps_scope_all_tasks() {
//     assert_eq!(
//         Target::parse("^:").unwrap(),
//         Target {
//             id: String::from("^:"),
//             scope: TargetScope::Deps,
//             task: TargetTask::All,
//         }
//     );
// }

#[test]
fn parse_self_scope() {
    assert_eq!(
        Target::parse("~:build").unwrap(),
        Target {
            id: CompactString::from("~:build"),
            scope: TargetScope::OwnSelf,
            task_id: Id::raw("build"),
        }
    );
}

#[test]
fn parse_self_when_no_colon() {
    assert_eq!(
        Target::parse("build").unwrap(),
        Target {
            id: CompactString::from("~:build"),
            scope: TargetScope::OwnSelf,
            task_id: Id::raw("build"),
        }
    );
}

// #[test]
// fn parse_self_scope_all_tasks() {
//     assert_eq!(
//         Target::parse("~:").unwrap(),
//         Target {
//             id: String::from("~:"),
//             scope: TargetScope::Own,
//             task: TargetTask::All,
//         }
//     );
// }

#[test]
fn parse_all_scopes() {
    assert_eq!(
        Target::parse(":build").unwrap(),
        Target {
            id: CompactString::from(":build"),
            scope: TargetScope::All,
            task_id: Id::raw("build"),
        }
    );
}

// #[test]
// fn parse_all_tasks() {
//     assert_eq!(
//         Target::parse("foo:").unwrap(),
//         Target {
//             id: String::from("foo:"),
//             scope: TargetScope::Id("foo".to_owned()),
//             task: TargetTask::All,
//         }
//     );
// }

#[test]
fn parse_node_package() {
    assert_eq!(
        Target::parse("@scope/foo:build").unwrap(),
        Target {
            id: CompactString::from("@scope/foo:build"),
            scope: TargetScope::Project(Id::raw("@scope/foo")),
            task_id: Id::raw("build"),
        }
    );
}

#[test]
fn parse_slashes() {
    assert_eq!(
        Target::parse("foo/sub:build/esm").unwrap(),
        Target {
            id: CompactString::from("foo/sub:build/esm"),
            scope: TargetScope::Project(Id::raw("foo/sub")),
            task_id: Id::raw("build/esm"),
        }
    );
}

#[test]
fn matches_all() {
    let all = Target::parse(":lint").unwrap();

    assert!(all.is_all_task("lint"));
    assert!(all.is_all_task(":lint"));
    assert!(!all.is_all_task("build"));
    assert!(!all.is_all_task(":build"));
    assert!(!all.is_all_task("foo:lint"));

    let full = Target::parse("foo:lint").unwrap();

    assert!(!full.is_all_task("lint"));
    assert!(!full.is_all_task(":lint"));
    assert!(!full.is_all_task("build"));
    assert!(!full.is_all_task(":build"));
    assert!(!full.is_all_task("foo:lint"));
}
