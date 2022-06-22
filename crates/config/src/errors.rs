use figment::error::Kind;
use figment::Error as FigmentError;
use serde_yaml::{to_value, Value};
use std::borrow::Cow;
use validator::{ValidationError, ValidationErrors, ValidationErrorsKind};

pub fn create_validation_error(code: &'static str, path: &str, message: String) -> ValidationError {
    let mut error = ValidationError::new(code);
    error.message = Some(Cow::from(message));
    // Is there a better way to do this?
    error.add_param(Cow::from("path"), &path.to_owned());
    error
}

fn format_yaml_value(value: Value) -> String {
    match value {
        Value::Null => String::from("null"),
        Value::Bool(_) => String::from("boolean"),
        Value::Number(_) => String::from("number"),
        Value::String(msg) => msg,
        Value::Sequence(array) => format!("array of {:?}", array),
        Value::Mapping(object) => format!("object of {:?}", object),
    }
}

fn format_validation_error(error: &ValidationError) -> String {
    let mut message = "".to_owned();

    if let Some(path) = error.params.get("path") {
        let value = format_yaml_value(to_value(&path).unwrap_or_default());

        if !value.is_empty() {
            let msg = format!("Invalid field <id>{}</id>: ", value);
            message.push_str(msg.as_str());
        }
    }

    if let Some(msg_internal) = &error.message {
        let msg = format!("{}", msg_internal);
        message.push_str(msg.as_str());
    } else {
        let msg = format!("Unknown failure [{}].", error.code);
        message.push_str(msg.as_str());
    }

    message
}

pub fn format_errors(errors: &ValidationErrors, indent: &str) -> String {
    let mut list = vec![];
    let mut push = |message: String, reset: bool| {
        if reset {
            list.push(message);
        } else {
            list.push(format!("{}<accent>▪</accent> {}", indent, message));
        }
    };

    for value in errors.errors().values() {
        match value {
            ValidationErrorsKind::Struct(obj) => {
                push(format_errors(obj, &format!("{}{}", indent, indent)), true);
            }
            ValidationErrorsKind::List(items) => {
                for item in items.values() {
                    push(format_errors(item, &format!("{}{}", indent, indent)), true);
                }
            }
            ValidationErrorsKind::Field(fields) => {
                for field in fields {
                    push(format_validation_error(field), false);
                }
            }
        }
    }

    list.join("\n")
}

pub fn map_figment_error_to_validation_errors(figment_error: &FigmentError) -> ValidationErrors {
    let path = figment_error.path.join(".");

    let valid_error = match &figment_error.kind {
        // Fields
        Kind::DuplicateField(field) => create_validation_error(
            "duplicate_field",
            path.as_str(),
            format!("Duplicate field <id>{}</id>.", field),
        ),
        Kind::MissingField(field) => create_validation_error(
            "missing_field",
            path.as_str(),
            format!("Missing field <id>{}</id>.", field),
        ),
        Kind::UnknownField(field, _) => create_validation_error(
            "unknown_field",
            path.as_str(),
            format!("Unknown field <id>{}</id>.", field),
        ),
        Kind::UnknownVariant(field, _) => create_validation_error(
            "unknown_field_variant",
            path.as_str(),
            format!("Unknown option <id>{}</id>.", field),
        ),

        // Values
        Kind::InvalidType(a, e) => create_validation_error(
            "invalid_type",
            path.as_str(),
            format!("Expected {} type, received {}.", e, a),
        ),
        Kind::InvalidLength(a, e) => create_validation_error(
            "invalid_length",
            path.as_str(),
            format!("Expected length of {}, received {}.", e, a),
        ),
        Kind::InvalidValue(a, e) => create_validation_error(
            "invalid_value",
            path.as_str(),
            format!("Expected {} value, received {}.", e, a),
        ),
        Kind::ISizeOutOfRange(range) => create_validation_error(
            "out_of_range",
            path.as_str(),
            format!("Integer out of range, received {}.", range),
        ),
        Kind::USizeOutOfRange(range) => create_validation_error(
            "out_of_range",
            path.as_str(),
            format!("Unsigned integer out of range, received {}.", range),
        ),

        // Other
        Kind::Message(message) => {
            create_validation_error("message", path.as_str(), String::from(message))
        }
        Kind::Unsupported(a) => create_validation_error(
            "unsupported",
            path.as_str(),
            format!("Unsupported type/value <muted>{}</muted>.", a),
        ),
        Kind::UnsupportedKey(key, _) => create_validation_error(
            "unsupported_key",
            path.as_str(),
            format!("Unsupported key <symbol>{}</symbol>.", key),
        ),
    };

    let mut errors = ValidationErrors::new();

    // We basically need a string literal here, but the path is dynamically provided...
    // https://stackoverflow.com/a/52367953
    errors.add(
        Box::leak(figment_error.path.join(".").into_boxed_str()),
        valid_error,
    );

    errors
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use figment::error::Kind;
    use figment::Error;
    use validator::{ValidationErrors, ValidationErrorsKind};

    fn extract_first_error(errors: &ValidationErrors) -> String {
        for val in errors.errors().values() {
            match val {
                ValidationErrorsKind::Struct(obj) => {
                    let result = extract_first_error(obj);

                    if !result.is_empty() {
                        return result;
                    }
                }
                ValidationErrorsKind::List(list) => {
                    if !list.is_empty() {
                        let item = extract_first_error(list.values().next().unwrap());

                        if !item.is_empty() {
                            return item;
                        }
                    }
                }
                ValidationErrorsKind::Field(field) => {
                    if !field.is_empty() {
                        return format_validation_error(&field[0]);
                    }
                }
            }
        }

        String::from("")
    }

    pub fn handled_jailed_error(errors: &ValidationErrors) -> Error {
        Error::from(Kind::Message(extract_first_error(errors)))
    }
}
