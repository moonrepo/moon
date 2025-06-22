use crate::{config_struct, config_unit_enum};
use schematic::{Config, ConfigEnum, validate};

config_unit_enum!(
    /// The types of events in which to notify the terminal.
    #[derive(ConfigEnum)]
    pub enum NotifierTerminalToasts {
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
        /// Display a toast in the terminal for certain events.
        pub terminal_toasts: Option<NotifierTerminalToasts>,

        /// A secure URL in which to send webhooks to.
        #[setting(validate = validate::url_secure)]
        pub webhook_url: Option<String>,

        /// Whether webhook requests require acknowledgment (2xx response).
        #[setting(default = false)]
        pub webhook_acknowledge: bool,
    }
);
