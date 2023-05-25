use crate::portable_path::FilePath;
use schematic::{validate, Config};
use serde::Serialize;

fn default_templates<C>(_ctx: &C) -> Option<Vec<FilePath>> {
    Some(vec![FilePath("./templates".into())])
}

#[derive(Clone, Config, Serialize)]
pub struct GeneratorConfig {
    #[setting(
        validate = validate::not_empty,
        default = default_templates
    )]
    pub templates: Vec<FilePath>,
}
