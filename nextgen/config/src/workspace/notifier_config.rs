use schematic::{validate, Config};

/// Configures how and where notifications are sent.
#[derive(Clone, Config, Debug)]
pub struct NotifierConfig {
    #[setting(validate = validate::url_secure)]
    pub webhook_url: Option<String>,
}
