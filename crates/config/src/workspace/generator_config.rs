use crate::config_struct;
use crate::portable_path::FilePath;
use crate::template::TemplateLocator;
use schematic::{Config, DefaultValueResult, validate};

fn default_templates<C>(_ctx: &C) -> DefaultValueResult<Vec<TemplateLocator>> {
    Ok(Some(vec![TemplateLocator::File {
        path: FilePath("./templates".into()),
    }]))
}

config_struct!(
    /// Configures the generator for scaffolding from templates.
    #[derive(Config)]
    pub struct GeneratorConfig {
        /// The list of file paths, relative from the workspace root,
        /// in which to locate templates.
        #[setting(
        validate = validate::not_empty,
        default = default_templates
    )]
        pub templates: Vec<TemplateLocator>,
    }
);
