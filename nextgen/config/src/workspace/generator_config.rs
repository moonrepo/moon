use crate::relative_path::{FilePath, ProjectRelativePath};
use schematic::{validate, Config};

fn default_templates() -> Vec<ProjectRelativePath<FilePath>> {
    vec![ProjectRelativePath(FilePath("./templates".into()))]
}

#[derive(Config)]
pub struct GeneratorConfig {
    #[setting(
        validate = validate::not_empty,
        default_fn = default_templates
    )]
    pub templates: Vec<ProjectRelativePath<FilePath>>,
}
