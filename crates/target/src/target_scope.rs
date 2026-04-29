use moon_common::Id;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum TargetDependencyScope {
    Build,
    Development,
    Peer,
    Production,
}

impl fmt::Display for TargetDependencyScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build => write!(f, "build"),
            Self::Development => write!(f, "development"),
            Self::Peer => write!(f, "peer"),
            Self::Production => write!(f, "production"),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum TargetProjectScope {
    All,                           // :task
    Deps,                          // ^:task
    DepsOf(TargetDependencyScope), // ^build:task, ^development:task, etc.
    Id,                            // project:task
    OwnSelf,                       // ~:task
    Tag,                           // #tag:task
}

impl TargetProjectScope {
    pub fn parse<T: AsRef<str>>(value: T) -> miette::Result<Self> {
        let value = match value.as_ref() {
            "" => Self::All,
            "^" => Self::Deps,
            "^build" => Self::DepsOf(TargetDependencyScope::Build),
            "^dev" | "^development" => Self::DepsOf(TargetDependencyScope::Development),
            "^peer" => Self::DepsOf(TargetDependencyScope::Peer),
            "^prod" | "^production" => Self::DepsOf(TargetDependencyScope::Production),
            "~" => Self::OwnSelf,
            id => {
                if let Some(id) = id.strip_prefix('#') {
                    Id::new(id)?;
                    Self::Tag
                } else {
                    Id::new(id)?;
                    Self::Id
                }
            }
        };

        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum TargetTaskScope {
    Id,  // project:task
    Tag, // project:#tag
}

impl TargetTaskScope {
    pub fn parse<T: AsRef<str>>(value: T) -> miette::Result<Self> {
        let value = value.as_ref();

        if let Some(id) = value.strip_prefix('#') {
            Id::new(id)?;
            Ok(Self::Tag)
        } else {
            Id::new(value)?;
            Ok(Self::Id)
        }
    }
}
