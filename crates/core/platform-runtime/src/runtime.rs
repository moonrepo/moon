use crate::version::Version;
use moon_config::PlatformType;
use serde::Serialize;
use std::fmt::{self, Debug};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "platform", content = "version")]
pub enum Runtime {
    Node(Version),
    System,
}

impl Runtime {
    pub fn label(&self) -> String {
        match self {
            Runtime::Node(version) => format!("Node.js v{version}"),
            Runtime::System => "system".into(),
        }
    }

    pub fn version(&self) -> Version {
        match self {
            Runtime::Node(version) => version.to_owned(),
            _ => Version::new("latest"),
        }
    }
}

impl fmt::Display for Runtime {
    // Primarily used in action graph node labels
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Runtime::Node(_) => write!(f, "Node"),
            Runtime::System => write!(f, "System"),
        }
    }
}

impl From<&Runtime> for PlatformType {
    fn from(value: &Runtime) -> Self {
        match value {
            Runtime::Node(_) => PlatformType::Node,
            Runtime::System => PlatformType::System,
        }
    }
}
