// package.json

use json;
use json::object::Object;
use json::JsonValue;
use std::fs;
use std::path::Path;

pub struct PackageJson;

impl PackageJson {
    pub fn load(path: &Path) -> Result<Object, json::Error> {
        match json::parse(fs::read_to_string(path).unwrap().as_str())? {
            JsonValue::Object(json) => Ok(json),
            _ => Err(json::Error::WrongType(String::from(
                "package.json must be an object",
            ))),
        }
    }

    pub fn save(path: &Path, json: JsonValue) -> Result<(), json::Error> {
        fs::write(path, json::stringify_pretty(json, 2)).unwrap();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;

    fn assert_load_package(string: &str, parsed: json::JsonValue) {
        let file = assert_fs::NamedTempFile::new("package.json").unwrap();
        file.write_str(string).unwrap();

        let package = PackageJson::load(file.path()).unwrap();

        assert_eq!(package, parsed);

        file.close().unwrap();
    }

    mod load {
        use super::*;
        use json::object;

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
    }

    mod save {}
}
