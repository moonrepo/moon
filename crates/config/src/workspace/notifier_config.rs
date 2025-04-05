use crate::config_struct;
use schematic::{Config, validate};

config_struct!(
    /// Configures how and where notifications are sent.
    #[derive(Config)]
    pub struct NotifierConfig {
        /// A secure URL in which to send webhooks to.
        #[setting(validate = validate::url_secure)]
        pub webhook_url: Option<String>,
    }
);
