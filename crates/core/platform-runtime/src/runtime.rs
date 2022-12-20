use serde::Serialize;
use std::fmt::{self, Debug};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Version(pub String, pub bool);

impl Version {
    pub fn is_override(&self) -> bool {
        self.1
    }
}

impl Default for Version {
    fn default() -> Self {
        Version("latest".into(), false)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "platform", content = "version")]
pub enum Runtime {
    Node(Version),
    System,
}

impl Runtime {
    pub fn label(&self) -> String {
        match self {
            Runtime::Node(version) => format!("Node.js v{}", version),
            Runtime::System => "system".into(),
        }
    }

    pub fn version(&self) -> Version {
        match self {
            Runtime::Node(version) => version.to_owned(),
            _ => Version("latest".into(), false),
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
