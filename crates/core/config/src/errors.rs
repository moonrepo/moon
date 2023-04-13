use figment::{Error as FigmentError, Figment};
use moon_error::MoonError;
use serde_json::Value;
use starbase_styles::{color, Style, Stylize};
use std::borrow::Cow;
use std::path::PathBuf;
use thiserror::Error;
use validator::{ValidationError, ValidationErrors, ValidationErrorsKind};

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to download extended configuration from {}.", .0.style(Style::Url))]
    FailedDownload(String),

    #[error("Failed validation.")]
    FailedValidation(Vec<FigmentError>),

    #[error("Invalid \"extends\" field, must be a string.")]
    InvalidExtendsField,

    #[error("Failed to parse YAML document {}: {1}", .0.style(Style::Path))]
    InvalidYaml(PathBuf, String),

    #[error("Cannot extend configuration file {} as it does not exist.", .0.style(Style::File))]
    MissingFile(String),

    #[error("Unable to extend {}, only YAML documents are supported.", .0.style(Style::File))]
    UnsupportedExtendsDocument(String),

    #[error("Cannot extend configuration file {}, only HTTPS URLs are supported.", .0.style(Style::File))]
    UnsupportedHttps(String),

    #[error(transparent)]
    Figment(#[from] FigmentError),

    #[error(transparent)]
    Moon(#[from] MoonError),
}

pub fn create_validation_error<P: AsRef<str>, M: AsRef<str>>(
    code: &'static str,
    path: P,
    message: M,
) -> ValidationError {
    let mut error = ValidationError::new(code);
    error.message = Some(Cow::from(message.as_ref().to_owned()));
    error.add_param(Cow::from("path"), &path.as_ref().to_owned());
    error
}

pub fn format_error_line<T: AsRef<str>>(msg: T) -> String {
    format!("  {} {}", color::muted("▪"), msg.as_ref())
}

pub fn format_figment_errors(errors: Vec<FigmentError>) -> String {
    let mut list = vec![];

    for error in errors {
        for nested_error in error {
            list.push(format_error_line(nested_error.to_string()));
        }
    }

    list.join("\n")
}

pub fn map_validation_errors_to_figment_errors(
    figment: &Figment,
    validation_errors: &ValidationErrors,
) -> Vec<FigmentError> {
    let mut errors = vec![];
    let mut nested_errors = vec![];

    let mut push_error = |validation_error: &ValidationError| {
        if validation_error.message.is_none() {
            return;
        }

        let mut figment_error = FigmentError::from(String::from(
            validation_error.message.as_ref().unwrap().clone(),
        ));

        figment_error.profile = Some(figment.profile().clone());

        if let Some(Value::String(path)) = validation_error.params.get("path") {
            if let Some(metadata) = figment.find_metadata(path) {
                figment_error.metadata = Some(metadata.clone());
            }

            figment_error = figment_error.with_path(path);
        };

        errors.push(figment_error);
    };

    for error_kind in validation_errors.errors().values() {
        match error_kind {
            ValidationErrorsKind::Struct(error) => {
                nested_errors.extend(map_validation_errors_to_figment_errors(figment, error));
            }
            ValidationErrorsKind::List(error_map) => {
                for error in error_map.values() {
                    nested_errors.extend(map_validation_errors_to_figment_errors(figment, error));
                }
            }
            ValidationErrorsKind::Field(error_list) => {
                for error in error_list {
                    push_error(error);
                }
            }
        }
    }

    errors.extend(nested_errors);
    errors
}
