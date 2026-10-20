use moon_common::Id;
use moon_config::{ProjectConfig, ToolchainsConfig, UnresolvedVersionSpec};
use serde::Serialize;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct ToolchainSpec {
    pub id: Id,
    pub req: Option<UnresolvedVersionSpec>,
}

impl ToolchainSpec {
    pub fn new(id: Id, req: UnresolvedVersionSpec) -> Self {
        Self { id, req: Some(req) }
    }

    pub fn new_global(id: Id) -> Self {
        Self { id, req: None }
    }

    pub fn system() -> Self {
        Self::new_global(Id::raw("system"))
    }

    pub fn is_global(&self) -> bool {
        self.req.is_none()
    }

    pub fn is_system(&self) -> bool {
        self.id == "system"
    }

    pub fn label(&self) -> String {
        if let Some(req) = &self.req {
            format!("{} {}", self.id, req)
        } else {
            self.id.to_string()
        }
    }

    pub fn id(&self) -> String {
        self.id.to_string()
    }

    pub fn target(&self) -> String {
        let mut key = self.id();

        if let Some(spec) = &self.req {
            let version = spec.to_string().replace(' ', "");

            key.push(':');
            key.push_str(&version);
        }

        key
    }
}

impl fmt::Display for ToolchainSpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.id())
    }
}

impl AsRef<ToolchainSpec> for ToolchainSpec {
    fn as_ref(&self) -> &ToolchainSpec {
        self
    }
}

/// The spec of a toolchain, from the workspace's toolchains config, which is
/// global when no version is configured, and `None` when the toolchain isn't
/// configured. When a project's config is provided, it may override the version,
/// or disable the toolchain, which is also `None`.
pub fn get_toolchain_spec(
    toolchains_config: &ToolchainsConfig,
    toolchain_id: &Id,
    project_config: Option<&ProjectConfig>,
) -> Option<ToolchainSpec> {
    if let Some(config) =
        project_config.and_then(|config| config.toolchains.get_plugin_config(toolchain_id))
    {
        if !config.is_enabled() {
            return None;
        }

        if let Some(version) = config.get_version() {
            return Some(ToolchainSpec::new(
                toolchain_id.to_owned(),
                version.to_owned(),
            ));
        }
    }

    toolchains_config
        .get_plugin_config(toolchain_id)
        .map(|config| match &config.version {
            Some(version) => ToolchainSpec::new(toolchain_id.to_owned(), version.to_owned()),
            None => ToolchainSpec::new_global(toolchain_id.to_owned()),
        })
}
