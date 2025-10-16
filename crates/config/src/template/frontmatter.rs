use crate::config_struct;
use schematic::Config;

config_struct!(
    /// Configures the leading frontmatter within a template file.
    /// Docs: https://moonrepo.dev/docs/config/template#frontmatter
    #[derive(Config)]
    pub struct TemplateFrontmatterConfig {
        #[setting(
            default = "https://moonrepo.dev/schemas/template-frontmatter.json",
            rename = "$schema"
        )]
        pub schema: String,

        /// Force overwrite a file at the destination if there is a conflict.
        pub force: bool,

        /// Override the destination using a relative file path.
        pub to: Option<String>,

        /// Skip writing this file to the destination.
        pub skip: bool,
    }
);

#[cfg(feature = "loader")]
impl TemplateFrontmatterConfig {
    pub fn parse<T: AsRef<str>>(content: T) -> miette::Result<TemplateFrontmatterConfig> {
        use moon_common::color;
        use schematic::{ConfigLoader, Format};

        let mut content = content.as_ref();

        if content.is_empty() {
            content = "{}";
        }

        let result = ConfigLoader::<TemplateFrontmatterConfig>::new()
            .set_help(color::muted_light(
                "https://moonrepo.dev/docs/config/template",
            ))
            .code(content, Format::Yaml)?
            .load()?;

        Ok(result.config)
    }
}
