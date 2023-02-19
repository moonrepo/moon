use crate::runtime::Runtime;
use serde::Serialize;
use std::fmt::{self, Debug};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Version {
    pub number: String,

    // Use version available on PATH
    pub path_executable: bool,

    // Is overriding the workspace version in a project
    pub project_override: bool,
}

impl Version {
    pub fn new(version: &str) -> Self {
        Version {
            number: version.to_owned(),
            path_executable: false,
            project_override: false,
        }
    }

    pub fn new_global() -> Self {
        Version {
            number: "global".into(),
            path_executable: true,
            project_override: false,
        }
    }

    pub fn new_override(version: &str) -> Self {
        Version {
            number: version.to_owned(),
            path_executable: false,
            project_override: true,
        }
    }

    pub fn is_global(&self) -> bool {
        self.number == "global" && self.path_executable
    }

    pub fn is_override(&self) -> bool {
        self.project_override
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.number)
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
