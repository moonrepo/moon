// tsconfig.json

use json::JsonValue;
use std::fs;
use std::path::{Path, PathBuf};

// We can't use serde_json here because:
//  - Additional or unknown fields are entirely lost,
//      which is problematic when we need to write back to the file.
//  - Field values are non-deterministic and can be _anything_,
//      which would result in parsing failures.
#[derive(Clone, Debug, PartialEq)]
pub struct TsconfigJson {
    pub path: PathBuf,

    pub value: json::object::Object,
}

impl TsconfigJson {
    pub fn load(path: &Path) -> Result<TsconfigJson, json::Error> {
        let contents = fs::read_to_string(path).unwrap();

        match json::parse(&contents)? {
            JsonValue::Object(value) => Ok(TsconfigJson {
                path: path.to_path_buf(),
                value,
            }),
            _ => Err(json::Error::WrongType(String::from(
                "Invalid `tsconfig.json`, must be an object.",
            ))),
        }
    }

    pub fn save(&self) -> Result<(), json::Error> {
        fs::write(&self.path, json::stringify_pretty(self.value.clone(), 2)).unwrap();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    mod load {
        use crate::TsconfigJson;
        use assert_fs::prelude::*;
        use json::object;

        fn assert_load_tsconfig(actual: &str, expected: json::JsonValue) {
            let file = assert_fs::NamedTempFile::new("tsconfig.json").unwrap();
            file.write_str(actual).unwrap();

            let tsconfig = TsconfigJson::load(file.path()).unwrap();

            assert_eq!(tsconfig, expected);

            file.close().unwrap();
        }

        #[test]
        fn loads_empty_file() {
            assert_load_tsconfig("{}", object! {});
        }

        #[test]
        fn doesnt_error_on_invalid_types() {
            assert_load_tsconfig(
                r#"{ "name": 123 }"#,
                object! {
                    name: 123,
                },
            );
        }

        #[test]
        fn allows_unknown_fields() {
            assert_load_tsconfig(
                r#"{ "unknown": "field" }"#,
                object! {
                    unknown: "field"
                },
            );
        }

        #[test]
        #[should_panic(expected = "Invalid `tsconfig.json`, must be an object.")]
        fn errors_if_nonobject() {
            assert_load_tsconfig(
                r#"123"#,
                object! {
                    unknown: "field"
                },
            );
        }
    }

    mod save {
        use crate::TsconfigJson;
        use json::object;
        use std::fs;

        fn assert_save_tsconfig(actual: json::JsonValue, expected: &str) {
            let file = assert_fs::NamedTempFile::new("tsconfig.json").unwrap();

            TsconfigJson::save(file.path(), actual).unwrap();

            assert_eq!(expected, fs::read_to_string(file.path()).unwrap());

            file.close().unwrap();
        }

        #[test]
        fn saves_file() {
            assert_save_tsconfig(
                object! {
                    compilerOptions: {
                        strict: true
                    },
                    extends: "./tsconfig.base.json"
                },
                r#"{
  "compilerOptions": {
    "strict": true
  },
  "extends": "./tsconfig.base.json"
}"#,
            );
        }
    }
}
