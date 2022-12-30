use serde::Serialize;
use std::{
    fmt::{self, Debug},
    str::FromStr,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Version(pub String, pub bool);

impl Version {
    pub fn new(version: &str) -> Self {
        Version(version.to_owned(), false)
    }

    pub fn new_override(version: &str) -> Self {
        Version(version.to_owned(), true)
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

// impl FromStr for Version {
//     type Err = ();

//     fn from_str(value: &str) -> Result<Self, Self::Err> {
//         Ok(Version::new(value))
//     }
// }
