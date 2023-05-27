use crate::version::Version;
use moon_config2::PlatformType;
use serde::Serialize;
use std::fmt::{self, Debug};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "platform", content = "version")]
pub enum Runtime {
    Deno(Version),
    Node(Version),
    Rust(Version),
    System,
}

impl Runtime {
    pub fn label(&self) -> String {
        match self {
            Runtime::Deno(version) => format!("Deno {version}"),
            Runtime::Node(version) => format!("Node.js {version}"),
            Runtime::Rust(version) => format!("Rust {version}"),
            Runtime::System => "system".into(),
        }
    }

    pub fn version(&self) -> Version {
        match self {
            Runtime::Deno(version) | Runtime::Node(version) | Runtime::Rust(version) => {
                version.to_owned()
            }
            Runtime::System => Version::new("latest"),
        }
    }
}

impl fmt::Display for Runtime {
    // Primarily used in action graph node labels
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Runtime::Deno(_) => write!(f, "Deno"),
            Runtime::Node(_) => write!(f, "Node"),
            Runtime::Rust(_) => write!(f, "Rust"),
            Runtime::System => write!(f, "System"),
        }
    }
}

impl From<&Runtime> for PlatformType {
    fn from(value: &Runtime) -> Self {
        match value {
            Runtime::Deno(_) => PlatformType::Deno,
            Runtime::Node(_) => PlatformType::Node,
            Runtime::Rust(_) => PlatformType::Rust,
            Runtime::System => PlatformType::System,
        }
    }
}
