pub use moon_config::{PlatformType, SemVer, UnresolvedVersionSpec, Version, VersionSpec};
use serde::Serialize;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RuntimeReq {
    // Use tool available on PATH
    Global,
    // Install tool into toolchain
    Toolchain(UnresolvedVersionSpec),
}

impl RuntimeReq {
    pub fn is_global(&self) -> bool {
        matches!(self, Self::Global)
    }

    pub fn to_spec(&self) -> Option<UnresolvedVersionSpec> {
        match self {
            Self::Toolchain(spec) => Some(spec.to_owned()),
            _ => None,
        }
    }
}

impl fmt::Display for RuntimeReq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Toolchain(spec) => write!(f, "{}", spec),
        }
    }
}

impl Hash for RuntimeReq {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Global => "global".hash(state),
            Self::Toolchain(spec) => match spec {
                UnresolvedVersionSpec::Canary => "canary".hash(state),
                UnresolvedVersionSpec::Alias(alias) => alias.hash(state),
                UnresolvedVersionSpec::Req(req) => req.hash(state),
                UnresolvedVersionSpec::ReqAny(reqs) => {
                    for req in reqs {
                        req.hash(state);
                    }
                }
                UnresolvedVersionSpec::Calendar(version) => version.hash(state),
                UnresolvedVersionSpec::Semantic(version) => version.hash(state),
            },
        };
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

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct Runtime {
    pub platform: PlatformType,
    pub requirement: RuntimeReq,
    pub overridden: bool,
}

impl Runtime {
    pub fn new(platform: PlatformType, requirement: RuntimeReq) -> Self {
        Self {
            platform,
            requirement,
            overridden: false,
        }
    }

    pub fn new_override(platform: PlatformType, requirement: RuntimeReq) -> Self {
        let mut runtime = Self::new(platform, requirement);
        runtime.overridden = true;
        runtime
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

    pub fn id(&self) -> String {
        self.platform.to_string().to_lowercase()
    }

    pub fn key(&self) -> String {
        let mut key = self.id();

        match &self.requirement {
            RuntimeReq::Global => {
                key.push_str(":global");
            }
            RuntimeReq::Toolchain(spec) => {
                let version = spec.to_string().replace(' ', "");

                key.push(':');
                key.push_str(&version);
            }
        };

        key
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
