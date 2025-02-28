use crate::{TargetScope, target::Target};
use moon_common::Id;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TargetLocator {
    // proj-*:task_id, proj:task-*
    GlobMatch {
        original: String,
        scope: Option<TargetScope>,
        project_glob: Option<String>,
        task_glob: String,
    },

    // scope:task_id
    Qualified(Target),

    // task_id
    TaskFromWorkingDir(Id),
}

impl TargetLocator {
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    #[tracing::instrument(name = "parse_target_locator")]
    pub fn parse(value: &str) -> miette::Result<TargetLocator> {
        Self::from_str(value)
    }
}

impl AsRef<TargetLocator> for TargetLocator {
    fn as_ref(&self) -> &TargetLocator {
        self
    }
}

impl AsRef<str> for TargetLocator {
    fn as_ref(&self) -> &str {
        match self {
            Self::GlobMatch { original, .. } => original.as_str(),
            Self::Qualified(target) => target.as_str(),
            Self::TaskFromWorkingDir(id) => id.as_str(),
        }
    }
}

impl PartialEq<Target> for TargetLocator {
    fn eq(&self, other: &Target) -> bool {
        match self {
            Self::Qualified(target) => target == other,
            _ => false,
        }
    }
}

impl FromStr for TargetLocator {
    type Err = miette::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.contains(':') {
            if is_glob(value) {
                let (base_scope, base_id) = value.split_once(':').unwrap();
                let mut scope = None;
                let mut project_glob = None;

                match base_scope {
                    "" | "*" | "**" | "**/*" => scope = Some(TargetScope::All),
                    "~" => scope = Some(TargetScope::OwnSelf),
                    "^" => scope = Some(TargetScope::Deps),
                    inner => {
                        project_glob = Some(inner.to_owned());
                    }
                };

                Ok(TargetLocator::GlobMatch {
                    original: value.to_owned(),
                    scope,
                    project_glob,
                    task_glob: base_id.to_owned(),
                })
            } else {
                Ok(TargetLocator::Qualified(Target::parse(value)?))
            }
        } else {
            Ok(TargetLocator::TaskFromWorkingDir(Id::new(value)?))
        }
    }
}

impl<'de> Deserialize<'de> for TargetLocator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        TargetLocator::from_str(&value).map_err(de::Error::custom)
    }
}

impl Serialize for TargetLocator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

fn is_glob(value: &str) -> bool {
    value.contains(['*', '?', '[', ']', '{', '}', '!'])
}
