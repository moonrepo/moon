use moon_common::Id;
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

    #[test]
    fn app_use_lib() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Library),
        )
        .unwrap();
    }

    #[test]
    fn app_use_tool() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Tool),
        )
        .unwrap();
    }

    #[test]
    fn app_use_unknown() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Unknown),
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type application")]
    fn app_cant_use_app() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Application),
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type application")]
    fn app_cant_use_e2e() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Application),
            &create_project("bar", ProjectType::Automation),
        )
        .unwrap();
    }

    #[test]
    fn lib_use_lib() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Library),
        )
        .unwrap();
    }

    #[test]
    fn lib_use_unknown() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Unknown),
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type library")]
    fn lib_cant_use_tool() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Tool),
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type library")]
    fn lib_cant_use_app() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Application),
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type library")]
    fn lib_cant_use_e2e() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Library),
            &create_project("bar", ProjectType::Automation),
        )
        .unwrap();
    }

    #[test]
    fn tool_use_lib() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Library),
        )
        .unwrap();
    }

    #[test]
    fn tool_use_unknown() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Unknown),
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type tool")]
    fn tool_cant_use_tool() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Tool),
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type tool")]
    fn tool_cant_use_app() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Application),
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type tool")]
    fn tool_cant_use_e2e() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Tool),
            &create_project("bar", ProjectType::Automation),
        )
        .unwrap();
    }

    #[test]
    fn e2e_use_app() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Application),
        )
        .unwrap();
    }

    #[test]
    fn e2e_use_lib() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Library),
        )
        .unwrap();
    }

    #[test]
    fn e2e_use_tool() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Tool),
        )
        .unwrap();
    }

    #[test]
    fn e2e_use_unknown() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Unknown),
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid project relationship. Project foo of type automation")]
    fn e2e_cant_use_e2e() {
        enforce_project_type_relationships(
            &create_project("foo", ProjectType::Automation),
            &create_project("bar", ProjectType::Automation),
        )
        .unwrap();
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
