use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("No template with the name <id>{0}</id> could be found at any of the defined template paths.")]
    MissingTemplate(String),
}
