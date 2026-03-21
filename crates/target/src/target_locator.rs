use crate::target::Target;
use crate::target_scope::TargetScope;
use moon_common::Id;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TargetLocator {
    // task_id
    DefaultProject(Id),

    // scope-*:task_id, scope:task-*
    GlobMatch {
        original: String,
        scope: Option<TargetScope>,
        scope_glob: Option<String>,
        task_glob: String,
    },

    // scope:task_id
    Qualified(Target),
}

impl TargetLocator {
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    #[tracing::instrument(name = "parse_target_locator")]
    pub fn parse(value: &str) -> miette::Result<TargetLocator> {
        if value.contains(':') {
            if value.contains(['*', '?', '[', ']', '{', '}', '!']) || value.contains("...") {
                let (base_scope, base_task) = value.split_once(':').unwrap();

                Ok(Self::parse_glob(value, base_scope, base_task)?)
            } else {
                Ok(TargetLocator::Qualified(Target::parse(value)?))
            }
        } else {
            Ok(TargetLocator::DefaultProject(Id::new(value)?))
        }
    }

    fn parse_glob(value: &str, base_scope: &str, base_task: &str) -> miette::Result<TargetLocator> {
        let mut scope = None;
        let mut scope_glob = None;

        match base_scope {
            "" | "*" | "**" | "**/*" | "..." => {
                scope = Some(TargetScope::All);
            }
            "~" | "^" | "^build" | "^dev" | "^development" | "^peer" | "^prod" | "^production" => {
                scope = Some(TargetScope::parse(base_scope)?);
            }
            inner => {
                scope_glob = Some(inner.replace("...", "**/*"));
            }
        };

        Ok(TargetLocator::GlobMatch {
            original: value.to_owned(),
            scope,
            scope_glob,
            task_glob: base_task.to_owned(),
        })
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
            Self::DefaultProject(id) => id.as_str(),
            Self::GlobMatch { original, .. } => original.as_str(),
            Self::Qualified(target) => target.as_str(),
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
        Self::parse(value)
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
