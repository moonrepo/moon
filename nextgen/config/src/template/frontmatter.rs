use moon_common::color;
use schematic::{Config, ConfigLoader, Format};

/// Docs: https://moonrepo.dev/docs/config/template#frontmatter
#[derive(Config, Debug, Eq, PartialEq)]
pub struct TemplateFrontmatterConfig {
    #[setting(
        default = "https://moonrepo.dev/schemas/template-frontmatter.json",
        rename = "$schema"
    )]
    pub schema: String,

    pub force: bool,
    pub to: Option<String>,
    pub skip: bool,
}

impl TemplateFrontmatterConfig {
    pub fn parse<T: AsRef<str>>(content: T) -> miette::Result<TemplateFrontmatterConfig> {
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
