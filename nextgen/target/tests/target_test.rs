use moon_target2::{Target, TargetScope};

#[test]
#[should_panic(expected = "InvalidFormat(\"foo$:build\")")]
fn invalid_chars() {
    Target::parse("foo$:build").unwrap();
}

#[test]
#[should_panic(expected = "InvalidFormat(\"foo:@build\")")]
fn invalid_task_no_at() {
    Target::parse("foo:@build").unwrap();
}

#[test]
fn format_all_scope() {
    assert_eq!(Target::format(TargetScope::All, "build").unwrap(), ":build");
}

#[test]
fn format_deps_scope() {
    assert_eq!(
        Target::format(TargetScope::Deps, "build").unwrap(),
        "^:build"
    );
}

#[test]
fn format_self_scope() {
    assert_eq!(
        Target::format(TargetScope::OwnSelf, "build").unwrap(),
        "~:build"
    );
}

#[test]
fn format_project_scope() {
    assert_eq!(
        Target::format(TargetScope::Project("foo".into()), "build").unwrap(),
        "foo:build"
    );
}

#[test]
fn format_tag_scope() {
    assert_eq!(
        Target::format(TargetScope::Tag("foo".into()), "build").unwrap(),
        "#foo:build"
    );
}

#[test]
fn format_with_slashes() {
    assert_eq!(
        Target::format(TargetScope::Project("foo/sub".into()), "build/esm").unwrap(),
        "foo/sub:build/esm"
    );
}

#[test]
fn format_node_package() {
    assert_eq!(
        Target::format(TargetScope::Project("@scope/foo".into()), "build").unwrap(),
        "@scope/foo:build"
    );
}

#[test]
fn parse_ids() {
    assert_eq!(
        Target::parse("foo:build").unwrap(),
        Target {
            id: String::from("foo:build"),
            scope: TargetScope::Project("foo".to_owned()),
            scope_id: Some("foo".to_owned()),
            task_id: "build".to_owned(),
            // task: TargetTask::Id("build".to_owned())
        }
    );
}

#[test]
fn parse_deps_scope() {
    assert_eq!(
        Target::parse("^:build").unwrap(),
        Target {
            id: String::from("^:build"),
            scope: TargetScope::Deps,
            scope_id: None,
            task_id: "build".to_owned(),
            // task: TargetTask::Id("build".to_owned())
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
            id: String::from("~:build"),
            scope: TargetScope::OwnSelf,
            scope_id: None,
            task_id: "build".to_owned(),
            // task: TargetTask::Id("build".to_owned())
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
            id: String::from(":build"),
            scope: TargetScope::All,
            scope_id: None,
            task_id: "build".to_owned(),
            // task: TargetTask::Id("build".to_owned())
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
#[should_panic(expected = "TooWild")]
fn parse_too_wild() {
    Target::parse(":").unwrap();
}

#[test]
fn parse_node_package() {
    assert_eq!(
        Target::parse("@scope/foo:build").unwrap(),
        Target {
            id: String::from("@scope/foo:build"),
            scope: TargetScope::Project("@scope/foo".to_owned()),
            scope_id: Some("@scope/foo".to_owned()),
            task_id: "build".to_owned(),
            // task: TargetTask::Id("build".to_owned())
        }
    );
}

#[test]
fn parse_slashes() {
    assert_eq!(
        Target::parse("foo/sub:build/esm").unwrap(),
        Target {
            id: String::from("foo/sub:build/esm"),
            scope: TargetScope::Project("foo/sub".to_owned()),
            scope_id: Some("foo/sub".to_owned()),
            task_id: "build/esm".to_owned(),
            // task: TargetTask::Id("build".to_owned())
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
