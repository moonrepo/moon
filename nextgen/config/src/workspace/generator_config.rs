use crate::portable_path::FilePath;
use schematic::{validate, Config};

fn default_templates<C>(_ctx: &C) -> Option<Vec<FilePath>> {
    Some(vec![FilePath("./templates".into())])
}

/// Configures the generator for scaffolding from templates.
#[derive(Clone, Config, Debug)]
pub struct GeneratorConfig {
    /// The list of file paths, relative from the workspace root,
    /// in which to locate templates.
    #[setting(
        validate = validate::not_empty,
        default = default_templates
    )]
    pub templates: Vec<FilePath>,
}
