use moon_common::Id;
use moon_config::{DependencyScope, StackType};
use moon_project::{Project, ProjectConfig, ProjectType};
use moon_project_constraints::{enforce_project_type_relationships, enforce_tag_relationships};

fn create_project(id: &str, type_of: ProjectType) -> Project {
    Project {
        id: Id::raw(id),
        type_of,
        ..Project::default()
    }
}

fn create_project_with_tags(id: &str, tags: Vec<Id>) -> Project {
    Project {
        id: Id::raw(id),
        config: ProjectConfig {
            tags,
            ..ProjectConfig::default()
        },
        ..Project::default()
    }
}

mod by_type {
    use super::*;

    mod scopes {
        use super::*;

        #[test]
        fn works_for_prod() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Application),
                &create_project("bar", ProjectType::Library),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        fn works_for_dev() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Application),
                &create_project("bar", ProjectType::Library),
                &DependencyScope::Development,
            )
            .unwrap();
        }

        #[test]
        fn works_for_peer() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Application),
                &create_project("bar", ProjectType::Library),
                &DependencyScope::Peer,
            )
            .unwrap();
        }

        #[test]
        fn works_for_build() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Application),
                &create_project("bar", ProjectType::Library),
                &DependencyScope::Build,
            )
            .unwrap();
        }

        #[test]
        fn works_for_root() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Application),
                &create_project("bar", ProjectType::Library),
                &DependencyScope::Root,
            )
            .unwrap();
        }

        #[test]
        fn doesnt_error_for_invalid_constraint_when_build() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Application),
                &create_project("bar", ProjectType::Application),
                &DependencyScope::Build,
            )
            .unwrap();
        }

        #[test]
        fn doesnt_error_for_invalid_constraint_when_root() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Application),
                &create_project("bar", ProjectType::Application),
                &DependencyScope::Root,
            )
            .unwrap();
        }
    }

    mod stacks {
        use super::*;

        #[test]
        fn doesnt_error_if_different_stack() {
            let mut a = create_project("foo", ProjectType::Application);
            a.config.stack = StackType::Frontend;

            let mut b = create_project("bar", ProjectType::Application);
            b.config.stack = StackType::Backend;

            enforce_project_type_relationships(&a, &b, &DependencyScope::Production).unwrap();
        }

        #[test]
        #[should_panic]
        fn errors_if_both_unknown_stack() {
            let mut a = create_project("foo", ProjectType::Application);
            a.config.stack = StackType::Unknown;

            let mut b = create_project("bar", ProjectType::Application);
            b.config.stack = StackType::Unknown;

            enforce_project_type_relationships(&a, &b, &DependencyScope::Production).unwrap();
        }

        #[test]
        #[should_panic]
        fn errors_if_unknown_and_other_stack() {
            let mut a = create_project("foo", ProjectType::Application);
            a.config.stack = StackType::Frontend;

            let mut b = create_project("bar", ProjectType::Application);
            b.config.stack = StackType::Unknown;

            enforce_project_type_relationships(&a, &b, &DependencyScope::Production).unwrap();
        }

        #[test]
        #[should_panic]
        fn errors_if_same_stack() {
            let mut a = create_project("foo", ProjectType::Application);
            a.config.stack = StackType::Frontend;

            let mut b = create_project("bar", ProjectType::Application);
            b.config.stack = StackType::Frontend;

            enforce_project_type_relationships(&a, &b, &DependencyScope::Production).unwrap();
        }
    }

    #[test]
    fn app_use_lib() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Library),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn app_use_tool() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Tool),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn app_use_config() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Configuration),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn app_use_scaffold() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Scaffolding),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn app_use_unknown() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Unknown),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type application")]
    fn app_cant_use_app() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Application),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type application")]
    fn app_cant_use_e2e() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Automation),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn lib_use_lib() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Library),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn lib_use_config() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Configuration),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn lib_use_scaffold() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Scaffolding),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn lib_use_unknown() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Unknown),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type library")]
    fn lib_cant_use_tool() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Tool),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type library")]
    fn lib_cant_use_app() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Application),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type library")]
    fn lib_cant_use_e2e() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Automation),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn tool_use_lib() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Library),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn tool_use_config() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Configuration),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn tool_use_scaffold() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Scaffolding),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn tool_use_unknown() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Unknown),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type tool")]
    fn tool_cant_use_tool() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Tool),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type tool")]
    fn tool_cant_use_app() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Application),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type tool")]
    fn tool_cant_use_e2e() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Automation),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn e2e_use_app() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Application),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn e2e_use_lib() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Library),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn e2e_use_tool() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Tool),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn e2e_use_config() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Configuration),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn e2e_use_scaffold() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Scaffolding),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    fn e2e_use_unknown() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Unknown),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type automation")]
    fn e2e_cant_use_e2e() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Automation),
            &DependencyScope::Production,
        )
        .unwrap();
    }

    mod config {
        use super::*;

        #[test]
        fn config_use_config() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Configuration),
                &create_project("bar", ProjectType::Configuration),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        fn config_use_scaffold() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Configuration),
                &create_project("bar", ProjectType::Scaffolding),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Invalid project relationship. Project foo of type configuration"
        )]
        fn config_cant_use_app() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Configuration),
                &create_project("bar", ProjectType::Application),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Invalid project relationship. Project foo of type configuration"
        )]
        fn config_cant_use_e2e() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Configuration),
                &create_project("bar", ProjectType::Automation),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Invalid project relationship. Project foo of type configuration"
        )]
        fn config_cant_use_lib() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Configuration),
                &create_project("bar", ProjectType::Library),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Invalid project relationship. Project foo of type configuration"
        )]
        fn config_cant_use_tool() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Configuration),
                &create_project("bar", ProjectType::Tool),
                &DependencyScope::Production,
            )
            .unwrap();
        }
    }

    mod scaffold {
        use super::*;

        #[test]
        fn scaffold_use_config() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Scaffolding),
                &create_project("bar", ProjectType::Configuration),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        fn scaffold_use_scaffold() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Scaffolding),
                &create_project("bar", ProjectType::Scaffolding),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "Invalid project relationship. Project foo of type scaffolding")]
        fn scaffold_cant_use_app() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Scaffolding),
                &create_project("bar", ProjectType::Application),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "Invalid project relationship. Project foo of type scaffolding")]
        fn scaffold_cant_use_e2e() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Scaffolding),
                &create_project("bar", ProjectType::Automation),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "Invalid project relationship. Project foo of type scaffolding")]
        fn scaffold_cant_use_lib() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Scaffolding),
                &create_project("bar", ProjectType::Library),
                &DependencyScope::Production,
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "Invalid project relationship. Project foo of type scaffolding")]
        fn scaffold_cant_use_tool() {
            enforce_project_type_relationships(
                &create_project("foo", ProjectType::Scaffolding),
                &create_project("bar", ProjectType::Tool),
                &DependencyScope::Production,
            )
            .unwrap();
        }
    }
}

mod by_tag {
    use super::*;

    #[test]
    fn allow_when_req_tags_empty() {
        enforce_tag_relationships(
            &create_project_with_tags("foo", vec![Id::raw("b")]),
            &Id::raw("a"),
            &create_project_with_tags("bar", vec![Id::raw("b")]),
            &[],
        )
        .unwrap();
    }

    #[test]
    fn allow_when_source_no_tags() {
        enforce_tag_relationships(
            &create_project_with_tags("foo", vec![]),
            &Id::raw("a"),
            &create_project_with_tags("bar", vec![]),
            &[Id::raw("c")],
        )
        .unwrap();
    }

    #[test]
    fn allow_when_source_not_have_tag() {
        enforce_tag_relationships(
            &create_project_with_tags("foo", vec![Id::raw("b")]),
            &Id::raw("a"),
            &create_project_with_tags("bar", vec![]),
            &[Id::raw("c")],
        )
        .unwrap();
    }

    #[test]
    fn allow_when_dep_has_source_tag() {
        enforce_tag_relationships(
            &create_project_with_tags("foo", vec![Id::raw("a")]),
            &Id::raw("a"),
            &create_project_with_tags("bar", vec![Id::raw("a")]),
            &[Id::raw("c")],
        )
        .unwrap();
    }

    #[test]
    fn allow_when_dep_has_req_tag() {
        enforce_tag_relationships(
            &create_project_with_tags("foo", vec![Id::raw("a")]),
            &Id::raw("a"),
            &create_project_with_tags("bar", vec![Id::raw("c")]),
            &[Id::raw("c")],
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid tag relationship. Project foo with tag #a cannot depend on")]
    fn fail_when_dep_has_no_matching_tags() {
        enforce_tag_relationships(
            &create_project_with_tags("foo", vec![Id::raw("a")]),
            &Id::raw("a"),
            &create_project_with_tags("bar", vec![Id::raw("b")]),
            &[Id::raw("c")],
        )
        .unwrap();
    }
}
