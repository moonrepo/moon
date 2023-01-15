use crate::errors::create_validation_error;
use moon_utils::regex::{matches_id, matches_target};
use moon_utils::semver::{Version, VersionReq};
use std::path::Path;
use validator::{validate_url as validate_base_url, ValidationError};

pub fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    let def: T = Default::default();
    value == &def
}

// Validate the value is a valid semver version/range.
pub fn validate_semver_version<K: AsRef<str>, V: AsRef<str>>(
    key: K,
    value: V,
) -> Result<(), ValidationError> {
    if Version::parse(value.as_ref()).is_err() {
        return Err(create_validation_error(
            "invalid_semver",
            key.as_ref(),
            "Must be a valid semantic version",
        ));
    }

    Ok(())
}

pub fn validate_semver_requirement<K: AsRef<str>, V: AsRef<str>>(
    key: K,
    value: V,
) -> Result<(), ValidationError> {
    if VersionReq::parse(value.as_ref()).is_err() {
        return Err(create_validation_error(
            "invalid_semver_req",
            key.as_ref(),
            "Must be a valid semantic version requirement or range",
        ));
    }

    Ok(())
}

// Validate the value is a valid child relative file system path.
// Will fail on absolute paths ("/"), and parent relative paths ("../").
pub fn validate_child_relative_path<K: AsRef<str>, V: AsRef<str>>(
    key: K,
    value: V,
) -> Result<(), ValidationError> {
    let key = key.as_ref();
    let path = Path::new(value.as_ref());

    if path.has_root() || path.is_absolute() {
        return Err(create_validation_error(
            "no_absolute",
            key,
            "Absolute paths are not supported",
        ));
    } else if path.starts_with("..") {
        return Err(create_validation_error(
            "no_parent_relative",
            key,
            "Parent relative paths are not supported",
        ));
    }

    Ok(())
}

// Validate the value is a valid child relative file system path or root path.
// Will fail on parent relative paths ("../") and absolute paths.
pub fn validate_child_or_root_path<K: AsRef<str>, V: AsRef<str>>(
    key: K,
    value: V,
) -> Result<(), ValidationError> {
    let key = key.as_ref();
    let path = Path::new(value.as_ref());

    if (path.has_root() || path.is_absolute()) && !path.starts_with("/") {
        return Err(create_validation_error(
            "no_absolute",
            key,
            "Absolute paths are not supported (root paths must start with \"/\")",
        ));
    } else if path.starts_with("..") {
        return Err(create_validation_error(
            "no_parent_relative",
            key,
            "Parent relative paths are not supported",
        ));
    }

    Ok(())
}

// Validate the value is a project ID, task ID, file group, etc.
pub fn validate_id<K: AsRef<str>, V: AsRef<str>>(key: K, id: V) -> Result<(), ValidationError> {
    if !matches_id(id.as_ref()) {
        return Err(create_validation_error(
            "invalid_id",
            key.as_ref(),
            "Must be a valid ID (accepts A-Z, a-z, 0-9, - (dashes), _ (underscores), /, and must start with a letter)",
        ));
    }

    Ok(())
}

// Validate the value is a target in the format of "project_id:task_id".
pub fn validate_target<K: AsRef<str>, V: AsRef<str>>(
    key: K,
    target_id: V,
) -> Result<(), ValidationError> {
    if !matches_target(target_id.as_ref()) {
        return Err(create_validation_error(
            "invalid_target",
            key.as_ref(),
            "Must be a valid target format",
        ));
    }

    Ok(())
}

// Validate the value is a URL, and optionally check if HTTPS.
pub fn validate_url<K: AsRef<str>, V: AsRef<str>>(
    key: K,
    value: V,
    https_only: bool,
) -> Result<(), ValidationError> {
    let key = key.as_ref();
    let value = value.as_ref();

    if !validate_base_url(value) || !value.starts_with("http") {
        return Err(create_validation_error(
            "invalid_url",
            key,
            "Must be a valid URL",
        ));
    }

    if https_only && !value.starts_with("https://") {
        return Err(create_validation_error(
            "invalid_https_url",
            key,
            "Only HTTPS URLs are supported",
        ));
    }

    Ok(())
}

// Validate the value is an acceptable URL or file path for an "extends" YAML field.
pub fn validate_extends<V: AsRef<str>>(value: V) -> Result<(), ValidationError> {
    let value = value.as_ref();

    if value.starts_with("http") {
        validate_url("extends", value, true)?;

        // Is there a better way to check that a value is a file system path?
        // We can't use existence checks because it's not absolute, and
        // we don't have a working directory to prefix the value with.
    } else if !value.starts_with('.') {
        return Err(create_validation_error(
            "unknown_format",
            "extends",
            "Must be a valid URL or relative file path (starts with ./)",
        ));
    }

    if !value.ends_with(".yml") && !value.ends_with(".yaml") {
        return Err(create_validation_error(
            "invalid_yaml",
            "extends",
            "Must be a YAML document",
        ));
    }

    Ok(())
}

// Validate the value is a non-empty string.
pub fn validate_non_empty<K: AsRef<str>, V: AsRef<str>>(
    key: K,
    value: V,
) -> Result<(), ValidationError> {
    if value.as_ref().is_empty() {
        return Err(create_validation_error(
            "non_empty",
            key.as_ref(),
            "Must be a non-empty string",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod validate_semver_version {
        use super::*;

        #[test]
        fn passes_for_valid() {
            assert!(validate_semver_version("key", "1.2.3").is_ok());
        }

        #[test]
        fn fails_for_invalid() {
            assert!(validate_semver_version("key", "1.2.abc").is_err());
        }
    }

    mod validate_child_relative_path {
        use super::*;

        #[test]
        fn passes_for_literal() {
            assert!(validate_child_relative_path("key", "file").is_ok());
        }

        #[test]
        fn passes_for_relative() {
            assert!(validate_child_relative_path("key", "./file").is_ok());
            assert!(validate_child_relative_path("key", ".\\file").is_ok());
        }

        #[cfg(not(windows))]
        #[test]
        fn fails_for_absolute() {
            assert!(validate_child_relative_path("key", "/file").is_err());
        }

        #[cfg(windows)]
        #[test]
        fn fails_for_absolute_windows() {
            assert!(validate_child_relative_path("key", "C:\\file").is_err());
        }

        #[cfg(not(windows))]
        #[test]
        fn fails_for_parent() {
            assert!(validate_child_relative_path("key", "../file").is_err());
        }

        #[cfg(windows)]
        #[test]
        fn fails_for_parent_windows() {
            assert!(validate_child_relative_path("key", "..\\file").is_err());
        }
    }

    mod validate_child_or_root_path {
        use super::*;

        #[test]
        fn passes_for_literal() {
            assert!(validate_child_or_root_path("key", "file").is_ok());
        }

        #[test]
        fn passes_for_relative() {
            assert!(validate_child_or_root_path("key", "./file").is_ok());
            assert!(validate_child_or_root_path("key", ".\\file").is_ok());
        }

        #[cfg(not(windows))]
        #[test]
        fn fails_for_absolute() {
            assert!(validate_child_or_root_path("key", "/file").is_ok());
        }

        #[cfg(windows)]
        #[test]
        fn fails_for_absolute_windows() {
            assert!(validate_child_or_root_path("key", "C:\\file").is_err());
        }

        #[cfg(not(windows))]
        #[test]
        fn fails_for_parent() {
            assert!(validate_child_or_root_path("key", "../file").is_err());
        }

        #[cfg(windows)]
        #[test]
        fn fails_for_parent_windows() {
            assert!(validate_child_or_root_path("key", "..\\file").is_err());
        }
    }

    mod validate_id {
        use super::*;

        #[test]
        fn supports_1_char() {
            assert!(validate_id("key", "a").is_ok());
        }

        #[test]
        fn supports_cases() {
            assert!(validate_id("key", "foo-bar1").is_ok());
            assert!(validate_id("key", "foo_bar2").is_ok());
            assert!(validate_id("key", "fooBar3").is_ok());
            assert!(validate_id("key", "foo-barBaz_qux4").is_ok());
        }

        #[test]
        fn fails_if_starts_with_nonchar() {
            assert!(validate_id("key", "1foo").is_err());
            assert!(validate_id("key", "-foo").is_err());
            assert!(validate_id("key", "_foo").is_err());
        }
    }

    mod validate_target {
        use super::*;

        #[test]
        fn supports_1_char() {
            assert!(validate_target("key", ":a").is_ok());
            assert!(validate_target("key", "a:b").is_ok());
        }

        #[test]
        fn supports_cases() {
            assert!(validate_target("key", "foo-bar1:abc").is_ok());
            assert!(validate_target("key", "foo_bar2:a-b").is_ok());
            assert!(validate_target("key", "fooBar3:cD").is_ok());
            assert!(validate_target("key", ":foo-barBaz_qux4").is_ok());
        }

        #[test]
        fn supports_project_scopes() {
            assert!(validate_target("key", "^:a").is_ok());
            assert!(validate_target("key", "~:b").is_ok());
            assert!(validate_target("key", ":c").is_ok());
        }

        #[test]
        fn fails_if_starts_with_nonchar() {
            assert!(validate_target("key", "1foo").is_err());
            assert!(validate_target("key", "-foo").is_err());
            assert!(validate_target("key", "_foo").is_err());
        }

        #[test]
        fn fails_if_task_uses_project_scopes() {
            assert!(validate_target("key", "a:^").is_err());
            assert!(validate_target("key", "b:~").is_err());
            assert!(validate_target("key", "c:").is_err());
        }
    }

    mod validate_url {
        use super::*;

        #[test]
        fn passes_for_url() {
            assert!(validate_url("key", "http://domain.com", false).is_ok());
            assert!(validate_url("key", "https://domain.com", false).is_ok());
        }

        #[test]
        fn fails_for_non_http() {
            assert!(validate_url("key", "ftp://domain.com", false).is_err());
            assert!(validate_url("key", "127.0.0.1", false).is_err());
        }

        #[test]
        fn fails_for_http_url_if_https_only() {
            assert!(validate_url("key", "http://domain.com", true).is_err());
        }

        #[test]
        fn fails_for_non_url() {
            assert!(validate_url("key", "domain.com", false).is_err());
            assert!(validate_url("key", "random value", false).is_err());
        }
    }
}
