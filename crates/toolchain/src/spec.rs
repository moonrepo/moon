use moon_common::Id;
use moon_config::UnresolvedVersionSpec;
use serde::Serialize;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct ToolchainSpec {
    pub id: Id,
    pub overridden: bool,
    pub req: Option<UnresolvedVersionSpec>,
}

impl ToolchainSpec {
    pub fn new(id: Id, req: UnresolvedVersionSpec) -> Self {
        Self {
            id,
            req: Some(req),
            overridden: false,
        }
    }

    pub fn new_global(id: Id) -> Self {
        Self {
            id,
            req: None,
            overridden: false,
        }
    }

    pub fn new_override(id: Id, req: UnresolvedVersionSpec) -> Self {
        Self {
            id,
            req: Some(req),
            overridden: true,
        }
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
