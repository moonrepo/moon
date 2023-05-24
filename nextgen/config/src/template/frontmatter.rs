use schematic::{Config, ConfigError, ConfigLoader};

/// Docs: https://moonrepo.dev/docs/config/template#frontmatter
#[derive(Debug, Config, Eq, PartialEq)]
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
    pub fn parse<T: AsRef<str>>(content: T) -> Result<TemplateFrontmatterConfig, ConfigError> {
        let mut content = content.as_ref();

        if content.is_empty() {
            content = "{}";
        }

        let result = ConfigLoader::<TemplateFrontmatterConfig>::yaml()
            .code(content)?
            .load()?;

        Ok(result.config)
    }
}
