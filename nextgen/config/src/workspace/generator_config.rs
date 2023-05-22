use crate::relative_path::{FilePath, ProjectRelativePath};
use schematic::{validate, Config};

fn default_templates<C>(_ctx: &C) -> Option<Vec<ProjectRelativePath<FilePath>>> {
    Some(vec![ProjectRelativePath(FilePath("./templates".into()))])
}

#[derive(Config)]
pub struct GeneratorConfig {
    #[setting(
        validate = validate::not_empty,
        default = default_templates
    )]
    pub templates: Vec<ProjectRelativePath<FilePath>>,
}
