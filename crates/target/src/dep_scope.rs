use std::fmt::{self, Display};

/// The dependency scope for filtering in `^scope:task` targets.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum DependencyScope {
    Build,
    Development,
    Peer,
    Production,
}

impl Display for DependencyScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build => write!(f, "build"),
            Self::Development => write!(f, "development"),
            Self::Peer => write!(f, "peer"),
            Self::Production => write!(f, "production"),
        }
    }
}
