use crate::dep_scope::DependencyScope;
use moon_common::Id;
use std::fmt::{self, Display};
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum TargetScope {
    All,                     // :task
    Deps,                    // ^:task
    DepsOf(DependencyScope), // ^build:task, ^development:task, etc.
    OwnSelf,                 // ~:task
    Project(Id),             // project:task
    Tag(Id),                 // #tag:task
}

impl TargetScope {
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
                    Self::Project(Id::new(id)?)
                }
            }
        };

        Ok(scope)
    }
}

impl Display for TargetScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetScope::All => write!(f, ""),
            TargetScope::Deps => write!(f, "^"),
            TargetScope::DepsOf(scope) => write!(f, "^{scope}"),
            TargetScope::OwnSelf => write!(f, "~"),
            TargetScope::Project(id) => write!(f, "{id}"),
            TargetScope::Tag(id) => write!(f, "#{id}"),
        }
    }
}

impl AsRef<TargetScope> for TargetScope {
    fn as_ref(&self) -> &TargetScope {
        self
    }
}

impl FromStr for TargetScope {
    type Err = miette::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
