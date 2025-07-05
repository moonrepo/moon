use compact_str::CompactString;
use miette::Diagnostic;
use regex::Regex;
use schematic::{Schema, SchemaBuilder, Schematic};
use serde::{Deserialize, Deserializer, Serialize, de};
use starbase_styles::{Style, Stylize};
use std::sync::OnceLock;
use std::{borrow::Borrow, fmt, ops::Deref, str::FromStr};
use thiserror::Error;

// https://docs.rs/regex/latest/regex/#perl-character-classes-unicode-friendly
// `\w` is too broad, as it includes punctuation and other characters,
// so we need to be explicit with our Unicode character classes.
pub static ID_CHARS: &str = r"[\p{Alphabetic}\p{M}\p{Join_Control}\d/\._-]*";

pub static ID_PATTERN: OnceLock<Regex> = OnceLock::new();
pub static ID_CLEAN: OnceLock<Regex> = OnceLock::new();

#[derive(Error, Debug, Diagnostic)]
#[diagnostic(code(id::invalid_format))]
#[error("Invalid format for {}, may only contain alpha-numeric characters, dashes (-), slashes (/), underscores (_), and periods (.).", .0.style(Style::Id))]
pub struct IdError(String);

#[derive(Clone, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Id(CompactString);

impl Id {
    pub fn new<S: AsRef<str>>(id: S) -> Result<Id, IdError> {
        let id = id.as_ref();

        // The @ is to support npm package scopes!
        let pattern =
            ID_PATTERN.get_or_init(|| Regex::new(format!("^(@?{ID_CHARS})$").as_str()).unwrap());

        if !pattern.is_match(id) {
            return Err(IdError(id.to_owned()));
        }

        Ok(Self::raw(id))
    }

    pub fn clean<S: AsRef<str>>(id: S) -> Result<Id, IdError> {
        // Remove @ so node based IDs don't become prefixed
        // with a leading -, causing pattern failures
        let id = id.as_ref().replace('@', "");

        // This is to clean an ID and remove unwanted characters
        let pattern = ID_CLEAN.get_or_init(|| Regex::new(r"[^0-9A-Za-z/\._-]+").unwrap());

        Id::new(pattern.replace_all(&id, "-"))
    }

    pub fn raw<S: AsRef<str>>(id: S) -> Id {
        Id(CompactString::new(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Stylize for Id {
    fn style(&self, style: Style) -> String {
        self.to_string().style(style)
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// impl AsRef<String> for Id {
//     fn as_ref(&self) -> &String {
//         &self.0
//     }
// }

impl AsRef<Id> for Id {
    fn as_ref(&self) -> &Id {
        self
    }
}

impl Deref for Id {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<str> for Id {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Id {
    fn eq(&self, other: &&str) -> bool {
        self.0 == other
    }
}

impl PartialEq<String> for Id {
    fn eq(&self, other: &String) -> bool {
        self.0 == other
    }
}

// Allows strings to be used for collection keys

// impl Borrow<String> for Id {
//     fn borrow(&self) -> &String {
//         &self.0
//     }
// }

impl Borrow<str> for Id {
    fn borrow(&self) -> &str {
        &self.0
    }
}

// Parsing values

impl FromStr for Id {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Id::new(s)
    }
}

impl TryFrom<&str> for Id {
    type Error = IdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl TryFrom<String> for Id {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Id::new(String::deserialize(deserializer)?)
            .map_err(|error| de::Error::custom(error.to_string()))
    }
}

impl Schematic for Id {
    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.string_default()
    }
}
