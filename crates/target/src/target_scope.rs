use crate::dep_scope::DependencyScope;
use moon_common::Id;
use std::fmt::{self, Display};
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum TargetProjectScope {
    All,                     // :task
    Deps,                    // ^:task
    DepsOf(DependencyScope), // ^build:task, ^development:task, etc.
    Id(Id),                  // project:task
    OwnSelf,                 // ~:task
    Tag(Id),                 // #tag:task
}

impl TargetProjectScope {
    pub fn parse<T: AsRef<str>>(value: T) -> miette::Result<Self> {
        let scope = match value.as_ref() {
            "" => Self::All,
            "^" => Self::Deps,
            "^build" => Self::DepsOf(DependencyScope::Build),
            "^dev" | "^development" => Self::DepsOf(DependencyScope::Development),
            "^peer" => Self::DepsOf(DependencyScope::Peer),
            "^prod" | "^production" => Self::DepsOf(DependencyScope::Production),
            "~" => Self::OwnSelf,
            id => {
                if let Some(tag) = id.strip_prefix('#') {
                    Self::Tag(Id::new(tag)?)
                } else {
                    Self::Id(Id::new(id)?)
                }
            }
        };

        Ok(scope)
    }
}

impl Display for TargetProjectScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, ""),
            Self::Deps => write!(f, "^"),
            Self::DepsOf(scope) => write!(f, "^{scope}"),
            Self::Id(id) => write!(f, "{id}"),
            Self::OwnSelf => write!(f, "~"),
            Self::Tag(id) => write!(f, "#{id}"),
        }
    }
}

impl AsRef<TargetProjectScope> for TargetProjectScope {
    fn as_ref(&self) -> &TargetProjectScope {
        self
    }
}

impl FromStr for TargetProjectScope {
    type Err = miette::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum TargetTaskScope {
    Id(Id),  // project:task
    Tag(Id), // project:#tag
}

impl TargetTaskScope {
    pub fn parse<T: AsRef<str>>(value: T) -> miette::Result<Self> {
        let value = value.as_ref();

        let scope = if let Some(tag) = value.strip_prefix('#') {
            Self::Tag(Id::new(tag)?)
        } else {
            Self::Id(Id::new(value)?)
        };

        Ok(scope)
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Id(id) => id.as_str(),
            Self::Tag(id) => id.as_str(),
        }
    }
}

impl Display for TargetTaskScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => write!(f, "{id}"),
            Self::Tag(id) => write!(f, "#{id}"),
        }
    }
}

impl AsRef<TargetTaskScope> for TargetTaskScope {
    fn as_ref(&self) -> &TargetTaskScope {
        self
    }
}

impl AsRef<str> for TargetTaskScope {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for TargetTaskScope {
    type Err = miette::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
