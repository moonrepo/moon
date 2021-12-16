// package.json

use json;
use json::object::Object;
use json::JsonValue;
use std::fs;
use std::path::Path;

pub use json::object::Object as PackageJsonValue;

// We can't use serde_json here because:
//  - Additional or unknown fields are entirely lost,
//      which is problematic when we need to write back to the file.
//  - Field values are non-deterministic and can be _anything_,
//      which would result in parsing failures.
#[derive(Debug)]
pub struct PackageJson;

impl PackageJson {
    pub fn from(contents: &str) -> Result<Object, json::Error> {
        match json::parse(contents)? {
            JsonValue::Object(data) => Ok(data),
            _ => Err(json::Error::WrongType(String::from(
                "Invalid `package.json`, must be an object.",
            ))),
        }
    }

    pub fn load(path: &Path) -> Result<Object, json::Error> {
        PackageJson::from(fs::read_to_string(path).unwrap().as_str())
    }

    pub fn save(path: &Path, data: JsonValue) -> Result<(), json::Error> {
        fs::write(path, json::stringify_pretty(data, 2)).unwrap();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    mod load {
        use crate::PackageJson;
        use assert_fs::prelude::*;
        use json::object;

        fn assert_load_package(actual: &str, expected: json::JsonValue) {
            let file = assert_fs::NamedTempFile::new("package.json").unwrap();
            file.write_str(actual).unwrap();

            let package = PackageJson::load(file.path()).unwrap();

            assert_eq!(package, expected);

            file.close().unwrap();
        }

        #[test]
        fn loads_empty_file() {
            assert_load_package("{}", object! {});
        }

        #[test]
        fn doesnt_error_on_invalid_types() {
            assert_load_package(
                r#"{ "name": 123 }"#,
                object! {
                    name: 123,
                },
            );
        }

        #[test]
        fn allows_unknown_fields() {
            assert_load_package(
                r#"{ "unknown": "field" }"#,
                object! {
                    unknown: "field"
                },
            );
        }

        #[test]
        #[should_panic(expected = "Invalid `package.json`, must be an object.")]
        fn errors_if_nonobject() {
            assert_load_package(
                r#"123"#,
                object! {
                    unknown: "field"
                },
            );
        }
    }

    mod save {
        use crate::PackageJson;
        use json::object;
        use std::fs;

        fn assert_save_package(actual: json::JsonValue, expected: &str) {
            let file = assert_fs::NamedTempFile::new("package.json").unwrap();

            PackageJson::save(file.path(), actual).unwrap();

            assert_eq!(expected, fs::read_to_string(file.path()).unwrap());

            file.close().unwrap();
        }

        #[test]
        fn saves_file() {
            assert_save_package(
                object! {
                    name: "name",
                    version: "1.2.3"
                },
                r#"{
  "name": "name",
  "version": "1.2.3"
}"#,
            );
        }
    }
}
