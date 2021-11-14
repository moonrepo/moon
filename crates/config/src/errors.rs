use figment::error::Kind;
use figment::Error as FigmentError;
use serde_yaml::to_value;
use std::borrow::Cow;
use validator::{ValidationError, ValidationErrors};

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
			let mut error = ValidationError::new("other");
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
