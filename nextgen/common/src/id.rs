use once_cell::sync::Lazy;
use regex::Regex;
use starbase_styles::{Style, Stylize};
use std::{
    fmt::{self, Display},
    ops::Deref,
    str::FromStr,
};
use thiserror::Error;

pub static ID_CHARS: &str = r"[0-9A-Za-z/\._-]*";

pub static ID_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(&format!("^([A-Za-z]{{1}}{})$", ID_CHARS)).unwrap());

#[derive(Error, Debug)]
#[error("Invalid identifier {}. May only contain alpha-numeric characters, dashes (-), slashes (/), underscores (_), and dots (.).", .0.style(Style::Id))]
pub struct IdError(String);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(String);

impl Id {
    pub fn new<S: AsRef<str>>(id: S) -> Result<Id, IdError> {
        let id = id.as_ref();

        if !ID_PATTERN.is_match(id) {
            return Err(IdError(id.to_owned()));
        }

        Ok(Self::raw(id))
    }

    pub fn raw<S: AsRef<str>>(id: S) -> Id {
        Id(id.as_ref().to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<String> for Id {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl AsRef<Id> for Id {
    fn as_ref(&self) -> &Id {
        self
    }
}

impl Deref for Id {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<&str> for Id {
    fn eq(&self, other: &&str) -> bool {
        &self.0 == other
    }
}

impl PartialEq<String> for Id {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl FromStr for Id {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Id::new(s)
    }
}
