use crate::relative_path::{FilePath, ProjectPortablePath};
use schematic::{validate, Config};

fn default_templates<C>(_ctx: &C) -> Option<Vec<ProjectPortablePath<FilePath>>> {
    Some(vec![ProjectPortablePath(FilePath("./templates".into()))])
}

#[derive(Config)]
pub struct GeneratorConfig {
    #[setting(
        validate = validate::not_empty,
        default = default_templates
    )]
    pub templates: Vec<ProjectPortablePath<FilePath>>,
}
