use schematic::{validate, Config};

#[derive(Clone, Config, Debug)]
pub struct NotifierConfig {
    #[setting(validate = validate::url_secure)]
    pub webhook_url: Option<String>,
}
