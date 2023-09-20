use moon_config::{PlatformType, UnresolvedVersionSpec};
use serde::Serialize;
use std::fmt;

#[derive(Clone, Debug, Serialize)]
pub enum RuntimeRequirement {
    // Use tool available on PATH
    Global,
    // Install tool into toolchain
    Toolchain(UnresolvedVersionSpec),
    // Use toolchain but override the version
    ToolchainOverride(UnresolvedVersionSpec),
}

impl RuntimeRequirement {
    pub fn is_latest(&self) -> bool {
        match self {
            Self::Toolchain(UnresolvedVersionSpec::Alias(alias)) => alias == "latest",
            Self::ToolchainOverride(UnresolvedVersionSpec::Alias(alias)) => alias == "latest",
            _ => false,
        }
    }
}

impl fmt::Display for RuntimeRequirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Toolchain(spec) => write!(f, "{}", spec),
            Self::ToolchainOverride(spec) => write!(f, "{}", spec),
        }
    }
}

impl AsRef<RuntimeRequirement> for RuntimeRequirement {
    fn as_ref(&self) -> &RuntimeRequirement {
        self
    }
}

impl From<&Runtime> for RuntimeRequirement {
    fn from(value: &Runtime) -> Self {
        value.requirement.clone()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Runtime {
    pub platform: PlatformType,
    pub requirement: RuntimeRequirement,
}

impl Runtime {
    pub fn label(&self) -> String {
        match self.platform {
            PlatformType::System => "system".into(),
            platform => format!("{:?} {}", platform, self.requirement),
        }
    }
}

impl fmt::Display for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.platform)
    }
}

impl AsRef<Runtime> for Runtime {
    fn as_ref(&self) -> &Runtime {
        self
    }
}

impl From<&Runtime> for PlatformType {
    fn from(value: &Runtime) -> Self {
        value.platform
    }
}
