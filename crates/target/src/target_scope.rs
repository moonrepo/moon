use moon_common::Id;
use std::fmt::{self, Display};

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum TargetScope {
    All,         // :task
    Deps,        // ^:task
    OwnSelf,     // ~:task
    Project(Id), // project:task
    Tag(Id),     // #tag:task
}

impl Display for TargetScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetScope::All => write!(f, ""),
            TargetScope::Deps => write!(f, "^"),
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
