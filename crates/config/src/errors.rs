use figment::error::Kind;
use figment::Error as FigmentError;
use serde_yaml::{to_value, Value};
use std::borrow::Cow;
use validator::{ValidationError, ValidationErrors};

fn format_yaml_value(value: Value) -> String {
	match value {
		Value::Null => String::from("null"),
		Value::Bool(_) => String::from("boolean"),
		Value::Number(_) => String::from("number"),
		Value::String(msg) => String::from(msg),
		Value::Sequence(array) => format!("array of {:?}", array),
		Value::Mapping(object) => format!("object of {:?}", object),
	}
}

pub fn format_validation_error(error: &ValidationError) -> String {
	let param_value = |name| format_yaml_value(to_value(error.params.get(name).unwrap()).unwrap());

	match error.code.as_ref() {
		"duplicate_field" => format!("Duplicate field `{}`.", param_value("field")),
		"invalid_length" => format!(
			"Invalid length for field `{}`. Expected {}, received {}.",
			param_value("path"),
			param_value("expected"),
			param_value("actual")
		),
		"invalid_type" => format!(
			"Invalid type for field `{}`. Expected {}, received {}.",
			param_value("path"),
			param_value("expected"),
			param_value("actual")
		),
		"invalid_value" => format!(
			"Invalid value for field `{}`. Expected {}, received {}.",
			param_value("path"),
			param_value("expected"),
			param_value("actual")
		),
		"missing_field" => format!("Missing field `{}`.", param_value("field")),
		"out_of_range" => format!(
			"Integer out of range for field `{}`. Received {}.",
			param_value("path"),
			param_value("range")
		),
		"unknown_field" => format!("Unknown field `{}`.", param_value("field")),
		"unknown_field_variant" => format!("Unknown field variant `{}`.", param_value("field")),
		"unsupported_key" => format!("Unsupported key `{}`.", param_value("key")),
		"unsupported" => format!("Unsupported `{}`.", param_value("actual")),
		"message" => format!("{}", param_value("message")),
		code => format!(
			"Unknown failure for field `{}` [{}].",
			param_value("path"),
			code
		),
	}
}

pub fn map_figment_error_to_validation_errors(figment_error: &FigmentError) -> ValidationErrors {
	let mut valid_error = match &figment_error.kind {
		// Fields
		Kind::DuplicateField(field) => {
			let mut error = ValidationError::new("duplicate_field");
			error.add_param(Cow::from("field"), field);
			error
		}
		Kind::MissingField(field) => {
			let mut error = ValidationError::new("missing_field");
			error.add_param(Cow::from("field"), field);
			error
		}
		Kind::UnknownField(field, _) => {
			let mut error = ValidationError::new("unknown_field");
			error.add_param(Cow::from("field"), field);
			error
		}
		Kind::UnknownVariant(field, _) => {
			let mut error = ValidationError::new("unknown_field_variant");
			error.add_param(Cow::from("field"), field);
			error
		}

		// Values
		Kind::InvalidType(a, e) => {
			let mut error = ValidationError::new("invalid_type");
			error.add_param(Cow::from("actual"), &format!("{}", a));
			error.add_param(Cow::from("expected"), &to_value(e).unwrap());
			error
		}
		Kind::InvalidLength(a, e) => {
			let mut error = ValidationError::new("invalid_length");
			error.add_param(Cow::from("actual"), &format!("{}", a));
			error.add_param(Cow::from("expected"), &to_value(e).unwrap());
			error
		}
		Kind::InvalidValue(a, e) => {
			let mut error = ValidationError::new("invalid_value");
			error.add_param(Cow::from("actual"), &format!("{}", a));
			error.add_param(Cow::from("expected"), &to_value(e).unwrap());
			error
		}
		Kind::ISizeOutOfRange(range) => {
			let mut error = ValidationError::new("out_of_range");
			error.add_param(Cow::from("range"), &to_value(range).unwrap());
			error
		}
		Kind::USizeOutOfRange(range) => {
			let mut error = ValidationError::new("out_of_range");
			error.add_param(Cow::from("range"), &to_value(range).unwrap());
			error
		}

		// Other
		Kind::Message(message) => {
			let mut error = ValidationError::new("message");
			error.add_param(Cow::from("message"), message);
			error
		}
		Kind::Unsupported(a) => {
			let mut error = ValidationError::new("unsupported");
			error.add_param(Cow::from("actual"), &format!("{}", a));
			error
		}
		Kind::UnsupportedKey(key, _) => {
			let mut error = ValidationError::new("unsupported_key");
			error.add_param(Cow::from("key"), &format!("{}", key));
			error
		}
	};

	let path = figment_error.path.join(".");
	valid_error.add_param(Cow::from("path"), &to_value(&path).unwrap());

	let mut errors = ValidationErrors::new();

	// We basically need a string literal here, but the path is dynamically provided...
	// https://stackoverflow.com/a/52367953
	errors.add(Box::leak(String::from(path).into_boxed_str()), valid_error);

	errors
}
