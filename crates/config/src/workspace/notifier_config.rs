use crate::{config_struct, config_unit_enum, is_false};
use schematic::{Config, ConfigEnum, validate};

config_unit_enum!(
    /// The types of events in which to send notifications.
    #[derive(ConfigEnum)]
    pub enum NotifierEventType {
        /// Never toast.
        #[default]
        Never,

        /// On pipeline success or failure.
        Always,

        /// On pipeline failure.
        Failure,

        /// On pipeline success.
        Success,

        /// On each task failure.
        TaskFailure,
    }
);

config_struct!(
    /// Configures how and where notifications are sent.
    #[derive(Config)]
    pub struct NotifierConfig {
        /// Display an OS notification for certain action pipeline events.
        /// @since 1.38.0
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub terminal_notifications: Option<NotifierEventType>,

        /// A secure URL in which to send webhooks to.
        #[setting(validate = validate::url_secure)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub webhook_url: Option<String>,

        /// Whether webhook requests require acknowledgment (2xx response).
        /// @since 1.38.0
        #[serde(default, skip_serializing_if = "is_false")]
        pub webhook_acknowledge: bool,
    }
);
