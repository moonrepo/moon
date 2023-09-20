use moon_config::{PlatformType, Version, VersionSpec};
use serde::Serialize;
use std::fmt;

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum RuntimeReq {
    // Use tool available on PATH
    Global,
    // Install tool into toolchain
    Toolchain(VersionSpec),
    // Use toolchain but override the version
    ToolchainOverride(VersionSpec),
}

impl RuntimeReq {
    pub fn is_global(&self) -> bool {
        matches!(self, Self::Global)
    }

    pub fn is_latest(&self) -> bool {
        match self {
            Self::Toolchain(VersionSpec::Alias(alias))
            | Self::ToolchainOverride(VersionSpec::Alias(alias)) => alias == "latest",
            _ => false,
        }
    }

    pub fn is_override(&self) -> bool {
        matches!(self, Self::ToolchainOverride(_))
    }

    pub fn to_version(&self) -> Option<Version> {
        match self {
            Self::Toolchain(VersionSpec::Version(version))
            | Self::ToolchainOverride(VersionSpec::Version(version)) => Some(version.to_owned()),
            _ => None,
        }
    }
}

impl fmt::Display for RuntimeReq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Toolchain(spec) | Self::ToolchainOverride(spec) => write!(f, "{}", spec),
        }
    }
}

impl AsRef<RuntimeReq> for RuntimeReq {
    fn as_ref(&self) -> &RuntimeReq {
        self
    }
}

impl From<&Runtime> for RuntimeReq {
    fn from(value: &Runtime) -> Self {
        value.requirement.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Runtime {
    pub platform: PlatformType,
    pub requirement: RuntimeReq,
}

impl Runtime {
    pub fn new(platform: PlatformType, requirement: RuntimeReq) -> Self {
        Self {
            platform,
            requirement,
        }
    }

    pub fn system() -> Self {
        Self::new(PlatformType::System, RuntimeReq::Global)
    }

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
