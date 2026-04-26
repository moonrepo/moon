use compact_str::CompactString;
use moon_common::Id;
use moon_target::{DependencyScope, Target, TargetProjectScope, TargetTaskScope};

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
    assert_eq!(Target::format(TargetProjectScope::All, "build"), ":build");
}

#[test]
fn format_deps_scope() {
    assert_eq!(Target::format(TargetProjectScope::Deps, "build"), "^:build");
}

#[test]
fn format_deps_of_build_scope() {
    assert_eq!(
        Target::format(TargetProjectScope::DepsOf(DependencyScope::Build), "build"),
        "^build:build"
    );
}

#[test]
fn format_deps_of_development_scope() {
    assert_eq!(
        Target::format(
            TargetProjectScope::DepsOf(DependencyScope::Development),
            "build"
        ),
        "^development:build"
    );
}

#[test]
fn format_deps_of_peer_scope() {
    assert_eq!(
        Target::format(TargetProjectScope::DepsOf(DependencyScope::Peer), "build"),
        "^peer:build"
    );
}

#[test]
fn format_deps_of_production_scope() {
    assert_eq!(
        Target::format(
            TargetProjectScope::DepsOf(DependencyScope::Production),
            "build"
        ),
        "^production:build"
    );
}

#[test]
fn format_self_scope() {
    assert_eq!(
        Target::format(TargetProjectScope::OwnSelf, "build"),
        "~:build"
    );
}

#[test]
fn format_project_scope() {
    assert_eq!(
        Target::format(TargetProjectScope::Id(Id::raw("foo")), "build"),
        "foo:build"
    );
}

#[test]
fn format_tag_scope() {
    assert_eq!(
        Target::format(TargetProjectScope::Tag(Id::raw("foo")), "build"),
        "#foo:build"
    );
}

#[test]
fn format_with_slashes() {
    assert_eq!(
        Target::format(TargetProjectScope::Id(Id::raw("foo/sub")), "build/esm"),
        "foo/sub:build/esm"
    );
}

#[test]
fn format_node_package() {
    assert_eq!(
        Target::format(TargetProjectScope::Id(Id::raw("@scope/foo")), "build"),
        "@scope/foo:build"
    );
}

#[test]
fn parse_ids() {
    assert_eq!(
        Target::parse("foo:build").unwrap(),
        Target {
            id: CompactString::from("foo:build"),
            project: TargetProjectScope::Id(Id::raw("foo")),
            task: TargetTaskScope::Id(Id::raw("build")),
        }
    );
}

#[test]
fn parse_deps_scope() {
    assert_eq!(
        Target::parse("^:build").unwrap(),
        Target {
            id: CompactString::from("^:build"),
            project: TargetProjectScope::Deps,
            task: TargetTaskScope::Id(Id::raw("build")),
        }
    );
}

#[test]
fn parse_deps_of_build_scope() {
    assert_eq!(
        Target::parse("^build:lint").unwrap(),
        Target {
            id: CompactString::from("^build:lint"),
            project: TargetProjectScope::DepsOf(DependencyScope::Build),
            task: TargetTaskScope::Id(Id::raw("lint")),
        }
    );
}

#[test]
fn parse_deps_of_development_scope() {
    assert_eq!(
        Target::parse("^development:lint").unwrap(),
        Target {
            id: CompactString::from("^development:lint"),
            project: TargetProjectScope::DepsOf(DependencyScope::Development),
            task: TargetTaskScope::Id(Id::raw("lint")),
        }
    );
    assert_eq!(
        Target::parse("^dev:lint").unwrap(),
        Target {
            id: CompactString::from("^development:lint"),
            project: TargetProjectScope::DepsOf(DependencyScope::Development),
            task: TargetTaskScope::Id(Id::raw("lint")),
        }
    );
}

#[test]
fn parse_deps_of_peer_scope() {
    assert_eq!(
        Target::parse("^peer:lint").unwrap(),
        Target {
            id: CompactString::from("^peer:lint"),
            project: TargetProjectScope::DepsOf(DependencyScope::Peer),
            task: TargetTaskScope::Id(Id::raw("lint")),
        }
    );
}

#[test]
fn parse_deps_of_production_scope() {
    assert_eq!(
        Target::parse("^production:lint").unwrap(),
        Target {
            id: CompactString::from("^production:lint"),
            project: TargetProjectScope::DepsOf(DependencyScope::Production),
            task: TargetTaskScope::Id(Id::raw("lint")),
        }
    );
    assert_eq!(
        Target::parse("^prod:lint").unwrap(),
        Target {
            id: CompactString::from("^production:lint"),
            project: TargetProjectScope::DepsOf(DependencyScope::Production),
            task: TargetTaskScope::Id(Id::raw("lint")),
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
            project: TargetProjectScope::OwnSelf,
            task: TargetTaskScope::Id(Id::raw("build")),
        }
    );
}

#[test]
fn parse_self_when_no_colon() {
    assert_eq!(
        Target::parse("build").unwrap(),
        Target {
            id: CompactString::from("~:build"),
            project: TargetProjectScope::OwnSelf,
            task: TargetTaskScope::Id(Id::raw("build")),
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
            project: TargetProjectScope::All,
            task: TargetTaskScope::Id(Id::raw("build")),
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
            project: TargetProjectScope::Id(Id::raw("@scope/foo")),
            task: TargetTaskScope::Id(Id::raw("build")),
        }
    );
}

#[test]
fn parse_slashes() {
    assert_eq!(
        Target::parse("foo/sub:build/esm").unwrap(),
        Target {
            id: CompactString::from("foo/sub:build/esm"),
            project: TargetProjectScope::Id(Id::raw("foo/sub")),
            task: TargetTaskScope::Id(Id::raw("build/esm")),
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
