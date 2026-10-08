use moon_common::Id;
use moon_config::{LanguageType, LayerType, StackType};
use moon_project::Project;

mod project_serde {
    use super::*;

    fn create_project() -> Project {
        Project {
            id: Id::raw("project"),
            ..Default::default()
        }
    }

    #[test]
    fn doesnt_serialize_unknown_types() {
        let json = serde_json::to_value(create_project()).unwrap();
        let fields = json.as_object().unwrap();

        assert!(!fields.contains_key("language"));
        assert!(!fields.contains_key("layer"));
        assert!(!fields.contains_key("stack"));

        let project: Project = serde_json::from_value(json).unwrap();

        assert_eq!(project.language, LanguageType::Unknown);
        assert_eq!(project.layer, LayerType::Unknown);
        assert_eq!(project.stack, StackType::Unknown);
    }

    #[test]
    fn round_trips_known_types() {
        let project = Project {
            language: LanguageType::Rust,
            layer: LayerType::Library,
            stack: StackType::Backend,
            ..create_project()
        };

        let json = serde_json::to_value(&project).unwrap();

        assert_eq!(json["language"], "rust");
        assert_eq!(json["layer"], "library");
        assert_eq!(json["stack"], "backend");

        let project: Project = serde_json::from_value(json).unwrap();

        assert_eq!(project.language, LanguageType::Rust);
        assert_eq!(project.layer, LayerType::Library);
        assert_eq!(project.stack, StackType::Backend);
    }
}
