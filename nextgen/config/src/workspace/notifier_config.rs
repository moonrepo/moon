use schematic::{validate, Config};

#[derive(Debug, Config)]
pub struct NotifierConfig {
    #[setting(validate = validate::url_secure)]
    pub webhook_url: Option<String>,
}
