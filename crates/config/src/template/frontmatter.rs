use crate::{config_struct, is_false};
use schematic::Config;

config_struct!(
    /// Configures the leading frontmatter within a template file.
    /// Docs: https://moonrepo.dev/docs/config/template#frontmatter
    #[derive(Config)]
    #[serde(default)]
    pub struct TemplateFrontmatterConfig {
        #[serde(rename = "$schema")]
        pub schema: String,

        /// Force overwrite a file at the destination if there is a conflict.
        #[serde(skip_serializing_if = "is_false")]
        pub force: bool,

        /// Override the destination using a relative file path.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub to: Option<String>,

        /// Skip writing this file to the destination.
        #[serde(skip_serializing_if = "is_false")]
        pub skip: bool,
    }
);
