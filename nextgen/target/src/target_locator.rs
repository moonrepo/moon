use crate::target::Target;
use moon_common::Id;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TargetLocator {
    Qualified(Target),      // scope:task_id
    TaskFromWorkingDir(Id), // task_id
}

impl TargetLocator {
    pub fn as_str(&self) -> &str {
        self.as_ref()
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
            Self::Qualified(target) => target.as_str(),
            Self::TaskFromWorkingDir(id) => id.as_str(),
        }
    }
}

impl FromStr for TargetLocator {
    type Err = miette::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(if value.contains(':') {
            TargetLocator::Qualified(Target::parse(value)?)
        } else {
            TargetLocator::TaskFromWorkingDir(Id::new(value)?)
        })
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
