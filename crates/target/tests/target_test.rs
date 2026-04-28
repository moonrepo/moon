use compact_str::CompactString;
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
#[should_panic(expected = "Wildcard project and task scopes")]
fn errors_on_too_wild() {
    Target::parse(":").unwrap();
}

#[test]
fn parse_ids() {
    assert_eq!(
        Target::parse("foo:build").unwrap(),
        Target {
            id: CompactString::from("foo:build"),
            project: TargetProjectScope::Id,
            task: TargetTaskScope::Id,
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
            task: TargetTaskScope::Id,
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
            task: TargetTaskScope::Id,
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
            task: TargetTaskScope::Id,
        }
    );
    assert_eq!(
        Target::parse("^dev:lint").unwrap(),
        Target {
            id: CompactString::from("^development:lint"),
            project: TargetProjectScope::DepsOf(DependencyScope::Development),
            task: TargetTaskScope::Id,
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
            task: TargetTaskScope::Id,
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
            task: TargetTaskScope::Id,
        }
    );
    assert_eq!(
        Target::parse("^prod:lint").unwrap(),
        Target {
            id: CompactString::from("^production:lint"),
            project: TargetProjectScope::DepsOf(DependencyScope::Production),
            task: TargetTaskScope::Id,
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
            task: TargetTaskScope::Id,
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
            task: TargetTaskScope::Id,
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
            task: TargetTaskScope::Id,
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
            project: TargetProjectScope::Id,
            task: TargetTaskScope::Id,
        }
    );
}

#[test]
fn parse_slashes() {
    assert_eq!(
        Target::parse("foo/sub:build/esm").unwrap(),
        Target {
            id: CompactString::from("foo/sub:build/esm"),
            project: TargetProjectScope::Id,
            task: TargetTaskScope::Id,
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

// Tag-based task identifiers (e.g. `project:#tag`)

#[test]
fn parse_task_tag() {
    assert_eq!(
        Target::parse("foo:#lint").unwrap(),
        Target {
            id: CompactString::from("foo:#lint"),
            project: TargetProjectScope::Id,
            task: TargetTaskScope::Tag,
        }
    );
}

#[test]
fn parse_task_tag_with_all_scope() {
    assert_eq!(
        Target::parse(":#lint").unwrap(),
        Target {
            id: CompactString::from(":#lint"),
            project: TargetProjectScope::All,
            task: TargetTaskScope::Tag,
        }
    );
}

#[test]
fn parse_task_tag_with_self_scope() {
    assert_eq!(
        Target::parse("~:#lint").unwrap(),
        Target {
            id: CompactString::from("~:#lint"),
            project: TargetProjectScope::OwnSelf,
            task: TargetTaskScope::Tag,
        }
    );
}

#[test]
fn parse_task_tag_with_deps_scope() {
    assert_eq!(
        Target::parse("^:#lint").unwrap(),
        Target {
            id: CompactString::from("^:#lint"),
            project: TargetProjectScope::Deps,
            task: TargetTaskScope::Tag,
        }
    );
}

#[test]
fn parse_task_tag_with_deps_of_scope() {
    assert_eq!(
        Target::parse("^build:#lint").unwrap(),
        Target {
            id: CompactString::from("^build:#lint"),
            project: TargetProjectScope::DepsOf(DependencyScope::Build),
            task: TargetTaskScope::Tag,
        }
    );
}

#[test]
fn parse_task_tag_with_project_tag_scope() {
    assert_eq!(
        Target::parse("#ui:#lint").unwrap(),
        Target {
            id: CompactString::from("#ui:#lint"),
            project: TargetProjectScope::Tag,
            task: TargetTaskScope::Tag,
        }
    );
}

#[test]
fn parse_task_tag_with_node_package() {
    assert_eq!(
        Target::parse("@scope/foo:#lint").unwrap(),
        Target {
            id: CompactString::from("@scope/foo:#lint"),
            project: TargetProjectScope::Id,
            task: TargetTaskScope::Tag,
        }
    );
}

#[test]
fn parse_task_tag_with_slashes() {
    assert_eq!(
        Target::parse("foo/sub:#lint/all").unwrap(),
        Target {
            id: CompactString::from("foo/sub:#lint/all"),
            project: TargetProjectScope::Id,
            task: TargetTaskScope::Tag,
        }
    );
}

#[test]
fn parse_task_tag_when_no_colon() {
    assert_eq!(
        Target::parse("#lint").unwrap(),
        Target {
            id: CompactString::from("~:#lint"),
            project: TargetProjectScope::OwnSelf,
            task: TargetTaskScope::Tag,
        }
    );
}

#[test]
#[should_panic(expected = "Invalid target foo:#bad$tag")]
fn errors_on_invalid_task_tag_chars() {
    Target::parse("foo:#bad$tag").unwrap();
}

#[test]
#[should_panic(expected = "Invalid target foo:#")]
fn errors_on_empty_task_tag() {
    Target::parse("foo:#").unwrap();
}

#[test]
fn new_project_infers_task_tag() {
    let target = Target::new("foo", "#lint").unwrap();

    assert_eq!(target.id, "foo:#lint");
    assert_eq!(target.project, TargetProjectScope::Id);
    assert_eq!(target.task, TargetTaskScope::Tag);
}

#[test]
fn new_self_infers_task_tag() {
    let target = Target::new_self("#lint").unwrap();

    assert_eq!(target.id, "~:#lint");
    assert_eq!(target.project, TargetProjectScope::OwnSelf);
    assert_eq!(target.task, TargetTaskScope::Tag);
}

#[test]
fn new_project_tag_infers_task_tag() {
    let target = Target::parse("ui#lint").unwrap();

    assert_eq!(target.id, "#ui:#lint");
    assert_eq!(target.project, TargetProjectScope::Tag);
    assert_eq!(target.task, TargetTaskScope::Tag);
}

#[test]
fn get_task_tag_id_returns_tag() {
    let target = Target::parse("foo:#lint").unwrap();

    assert_eq!(target.get_task_tag(), Some("lint"));
}

#[test]
fn get_task_tag_id_returns_none_for_id_task() {
    let target = Target::parse("foo:lint").unwrap();

    assert_eq!(target.get_task_tag(), None);
}

#[test]
fn get_task_id_errors_when_task_is_tag() {
    let target = Target::parse("foo:#lint").unwrap();

    assert!(target.get_task_id().is_err());
}

#[test]
fn is_all_task_false_when_task_is_tag() {
    let target = Target::parse(":#lint").unwrap();

    assert!(!target.is_all_task("lint"));
    assert!(!target.is_all_task(":lint"));
    assert!(!target.is_all_task("#lint"));
    assert!(!target.is_all_task(":#lint"));
}
