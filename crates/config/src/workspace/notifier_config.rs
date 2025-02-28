use schematic::{Config, validate};

/// Configures how and where notifications are sent.
#[derive(Clone, Config, Debug, PartialEq)]
pub struct NotifierConfig {
    /// A secure URL in which to send webhooks to.
    #[setting(validate = validate::url_secure)]
    pub webhook_url: Option<String>,
}
