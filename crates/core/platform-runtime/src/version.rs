use crate::runtime::Runtime;
use serde::Serialize;
use std::fmt::{self, Debug};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Version(pub String, pub bool);

impl Version {
    pub fn new(version: &str) -> Self {
        Version(version.to_owned(), false)
    }

    pub fn new_override(version: &str) -> Self {
        Version(version.to_owned(), true)
    }

    pub fn is_latest(&self) -> bool {
        self.0 == "latest"
    }

    pub fn is_override(&self) -> bool {
        self.1
    }
}

impl Default for Version {
    fn default() -> Self {
        Version::new("latest")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Version {
    fn from(value: &str) -> Self {
        Version::new(value)
    }
}

impl From<&Runtime> for Version {
    fn from(value: &Runtime) -> Self {
        value.version()
    }
}

impl AsRef<Version> for Version {
    fn as_ref(&self) -> &Version {
        self
    }
}
