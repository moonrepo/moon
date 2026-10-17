use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_toolchain::{DependenciesWorkspaceRole, ToolchainSpec};
use rustc_hash::FxHashMap;
use serde::{Serialize, Serializer};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::sync::Arc;

// `Arc` is only serializable with serde's `rc` feature
fn serialize_config<S: Serializer>(
    config: &Arc<JsonValue>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    config.as_ref().serialize(serializer)
}

/// A label for a toolchain's root, like `node @ apps/web`, where the
/// workspace root is `.`.
pub fn get_root_label(toolchain: &Id, root: &WorkspaceRelativePathBuf) -> String {
    format!(
        "{toolchain} @ {}",
        if root.as_str().is_empty() {
            "."
        } else {
            root.as_str()
        }
    )
}

/// Identifies a toolchain dependencies root within a layout graph.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct RootIndex(pub(crate) u32);

impl RootIndex {
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RootKind {
    /// A workspace, with member globs relative to the root. The globs are kept,
    /// even though members are resolved to projects, as a dependencies workspace
    /// may have packages that aren't projects.
    Workspace { members: Vec<String> },

    /// A single package, that isn't a workspace.
    Package,
}

/// Files that exist in a root, probed from the toolchain's registered names.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootFiles {
    /// Every registered lockfile name that exists, as more than one can.
    pub lockfiles: Vec<WorkspaceRelativePathBuf>,
    pub manifests: Vec<WorkspaceRelativePathBuf>,
    pub vendor_dir: Option<WorkspaceRelativePathBuf>,
}

/// Functions that the toolchain's plugin supports for its roots.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootCapabilities {
    pub install_dependencies: bool,
    pub setup_environment: bool,
}

/// A directory located by a toolchain's `locate_dependencies_root`, which is
/// identified by the toolchain and its path.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainDependenciesRoot {
    /// The toolchain that located this root.
    pub toolchain: Id,
    pub root: WorkspaceRelativePathBuf,
    pub kind: RootKind,

    /// The project whose root is this root. Only projects in the workspace
    /// graph are known, so it's `None` when a partial graph doesn't include it.
    pub owner_project: Option<Id>,

    /// What this root's environment setup and install run with: the owner
    /// project's (including an overridden version) for a package, otherwise
    /// the workspace's.
    pub spec: ToolchainSpec,
    #[serde(serialize_with = "serialize_config")]
    pub config: Arc<JsonValue>,

    pub files: RootFiles,
    pub capabilities: RootCapabilities,

    /// Projects placed in this root, sorted. Only complete for a full build.
    pub members: Vec<Id>,
}

impl ToolchainDependenciesRoot {
    pub fn label(&self) -> String {
        get_root_label(&self.toolchain, &self.root)
    }
}

/// Where a binding's project lives for its toolchain.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Placement {
    /// In a dependencies root, with a role.
    Root {
        index: RootIndex,
        role: DependenciesWorkspaceRole,
    },

    /// Located a root, but isn't a member of it. No root is created for it.
    Outside { root: WorkspaceRelativePathBuf },

    /// The toolchain doesn't support locating a root (tier 2), or none was located.
    None,
}

/// A toolchain of a project (or of the workspace, without a project).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub toolchain: Id,

    /// The project's own spec, which its tasks run with, or the workspace's
    /// without a project.
    pub spec: ToolchainSpec,

    /// The workspace's config, merged with the project's.
    #[serde(serialize_with = "serialize_config")]
    pub config: Arc<JsonValue>,

    pub placement: Placement,
}

impl Binding {
    pub fn root_index(&self) -> Option<RootIndex> {
        match &self.placement {
            Placement::Root { index, .. } => Some(*index),
            _ => None,
        }
    }

    pub fn role(&self) -> Option<DependenciesWorkspaceRole> {
        match &self.placement {
            Placement::Root { role, .. } => Some(*role),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutGraph {
    pub(crate) roots: Vec<ToolchainDependenciesRoot>,

    #[serde(skip)]
    pub(crate) root_keys: FxHashMap<(Id, WorkspaceRelativePathBuf), RootIndex>,

    /// Per project, sorted by toolchain.
    pub(crate) bindings: BTreeMap<Id, Vec<Binding>>,

    /// Located from the workspace root without a project, sorted by toolchain.
    pub(crate) workspace_bindings: Vec<Binding>,

    /// Whether every project was placed, so that root members are complete.
    pub(crate) complete: bool,
}

impl LayoutGraph {
    /// The binding of a toolchain for a project.
    pub fn binding(&self, project_id: &Id, toolchain_id: &Id) -> Option<&Binding> {
        self.bindings(project_id)
            .iter()
            .find(|binding| &binding.toolchain == toolchain_id)
    }

    /// Every binding of a project, sorted by toolchain.
    pub fn bindings(&self, project_id: &Id) -> &[Binding] {
        self.bindings
            .get(project_id)
            .map(|bindings| bindings.as_slice())
            .unwrap_or_default()
    }

    /// The binding of a toolchain for the workspace, without a project.
    pub fn workspace_binding(&self, toolchain_id: &Id) -> Option<&Binding> {
        self.workspace_bindings
            .iter()
            .find(|binding| &binding.toolchain == toolchain_id)
    }

    /// The root at the index, which must be from this graph.
    pub fn root(&self, index: RootIndex) -> &ToolchainDependenciesRoot {
        &self.roots[index.index()]
    }

    /// The root that a project is placed in for a toolchain.
    pub fn root_for(
        &self,
        project_id: &Id,
        toolchain_id: &Id,
    ) -> Option<&ToolchainDependenciesRoot> {
        self.binding(project_id, toolchain_id)
            .and_then(|binding| binding.root_index())
            .map(|index| self.root(index))
    }

    /// The index of a toolchain's root at the path.
    pub fn find_root(
        &self,
        toolchain_id: &Id,
        root: &WorkspaceRelativePathBuf,
    ) -> Option<RootIndex> {
        self.root_keys
            .get(&(toolchain_id.to_owned(), root.to_owned()))
            .copied()
    }

    /// Every root, in the order they were located.
    pub fn roots(&self) -> &[ToolchainDependenciesRoot] {
        &self.roots
    }

    /// Every root located by a toolchain.
    pub fn roots_of<'graph>(
        &'graph self,
        toolchain_id: &'graph Id,
    ) -> impl Iterator<Item = &'graph ToolchainDependenciesRoot> {
        self.roots
            .iter()
            .filter(move |root| &root.toolchain == toolchain_id)
    }

    /// Every project's bindings, by project ID.
    pub fn iter_bindings(&self) -> impl Iterator<Item = (&Id, &[Binding])> {
        self.bindings
            .iter()
            .map(|(id, bindings)| (id, bindings.as_slice()))
    }

    /// Every binding of the workspace, without a project, sorted by toolchain.
    pub fn workspace_bindings(&self) -> &[Binding] {
        &self.workspace_bindings
    }

    /// Whether every project was placed, so that root members are complete.
    pub fn is_complete(&self) -> bool {
        self.complete
    }
}
