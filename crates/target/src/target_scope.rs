use crate::dep_scope::DependencyScope;

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum TargetProjectScope {
    All,                     // :task
    Deps,                    // ^:task
    DepsOf(DependencyScope), // ^build:task, ^development:task, etc.
    Id,                      // project:task
    OwnSelf,                 // ~:task
    Tag,                     // #tag:task
}

impl TargetProjectScope {
    pub fn parse<T: AsRef<str>>(value: T) -> Self {
        match value.as_ref() {
            "" => Self::All,
            "^" => Self::Deps,
            "^build" => Self::DepsOf(DependencyScope::Build),
            "^dev" | "^development" => Self::DepsOf(DependencyScope::Development),
            "^peer" => Self::DepsOf(DependencyScope::Peer),
            "^prod" | "^production" => Self::DepsOf(DependencyScope::Production),
            "~" => Self::OwnSelf,
            id => {
                if id.starts_with('#') {
                    Self::Tag
                } else {
                    Self::Id
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum TargetTaskScope {
    Id,  // project:task
    Tag, // project:#tag
}

impl TargetTaskScope {
    pub fn parse<T: AsRef<str>>(value: T) -> Self {
        if value.as_ref().starts_with('#') {
            Self::Tag
        } else {
            Self::Id
        }
    }
}
