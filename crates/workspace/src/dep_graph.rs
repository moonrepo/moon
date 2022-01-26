use crate::errors::WorkspaceError;
use moon_logger::{color, debug, trace};
use moon_project::{ProjectGraph, Target, TouchedFilePaths};
use petgraph::algo::toposort;
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use petgraph::Graph;
use std::collections::HashMap;

pub use petgraph::graph::NodeIndex;

pub enum NodeType {
    InstallNodeDeps,
    RunTarget(String), // target id
    SetupToolchain,
    SyncProject(String), // project id
}

pub struct Node {
    pub label: String,
    pub type_of: NodeType,
}

type GraphType = DiGraph<Node, u8>;

/// A directed acyclic graph (DAG) for the work that needs to be processed, based on a
/// project or task's dependency chain. This is also known as a "task graph" (not to
/// be confused with ours) or a "dependency graph". We call it a "work graph", as it's
/// the combination of those 2 with additional information for the work unit (a job).
pub struct DepGraph {
    graph: GraphType,

    /// Reference node for the "setup toolchain" job.
    toolchain_node: NodeIndex,

    /// Mapping of IDs to existing node indices.
    index_cache: HashMap<String, NodeIndex>,

    /// Reference node for the "install deps" job.
    install_deps_node: NodeIndex,

    /// Mapping of node indices to node data.
    nodes: HashMap<NodeIndex, NodeType>,
}

impl DepGraph {
    pub fn new() -> Self {
        debug!(
            target: "moon:dep-graph",
            "Creating work graph",
        );

        let mut graph: GraphType = Graph::new();
        let mut nodes: HashMap<NodeIndex, NodeType> = HashMap::new();

        // Toolchain must be setup first
        let toolchain_node = graph.add_node(Node {
            label: String::from(":setup_toolchain"),
            type_of: NodeType::SetupToolchain,
        });
        nodes.insert(toolchain_node, NodeType::SetupToolchain);

        // Deps can be installed *after* the toolchain exists
        let install_deps_node = graph.add_node(Node {
            label: String::from(":install_node_deps"),
            type_of: NodeType::InstallNodeDeps,
        });
        graph.add_edge(toolchain_node, install_deps_node, 0);
        nodes.insert(toolchain_node, NodeType::InstallNodeDeps);

        DepGraph {
            graph,
            index_cache: HashMap::new(),
            install_deps_node,
            nodes,
            toolchain_node,
        }
    }

    pub fn get_node_from_index(&self, index: NodeIndex) -> Option<&Node> {
        self.graph.node_weight(index)
    }

    pub fn sort_topological(&self) -> Result<Vec<NodeIndex>, WorkspaceError> {
        match toposort(&self.graph, None) {
            Ok(nodes) => Ok(nodes),
            Err(error) => Err(WorkspaceError::CycleDetected(error.node_id().index())),
        }
    }

    pub fn run_target(
        &mut self,
        target: &str,
        projects: &ProjectGraph,
    ) -> Result<NodeIndex, WorkspaceError> {
        if self.index_cache.contains_key(target) {
            return Ok(*self.index_cache.get(target).unwrap());
        }

        trace!(
            target: "moon:dep-graph",
            "Target {} does not exist in the work graph, inserting",
            color::id(target),
        );

        let (project_id, task_id) = Target::parse(target)?;
        let project = projects.get(&project_id)?;

        let node = self.graph.add_node(Node {
            label: target.to_owned(),
            type_of: NodeType::RunTarget(target.to_owned()),
        });
        self.graph.add_edge(self.install_deps_node, node, 0);
        self.nodes
            .insert(node, NodeType::RunTarget(target.to_owned()));

        // We should sync projects *before* running targets
        let project_node = self.sync_project(&project.id, projects)?;
        self.graph.add_edge(project_node, node, 0);

        // And we also need to wait on all dependent nodes
        let task = project.get_task(&task_id)?;

        if !task.deps.is_empty() {
            let dep_names: Vec<String> = task
                .deps
                .clone()
                .into_iter()
                .map(|d| color::symbol(&d))
                .collect();

            trace!(
                target: "moon:dep-graph",
                "Adding dependencies {} from target {}",
                dep_names.join(", "),
                color::id(target),
            );

            for dep_target in &task.deps {
                let dep_node = self.run_target(dep_target, projects)?;
                self.graph.add_edge(dep_node, node, 0);
            }
        }

        // Also cache so we don't run the same target multiple times
        self.index_cache.insert(target.to_owned(), node);

        Ok(node)
    }

    pub fn run_target_if_touched(
        &mut self,
        target: &str,
        touched_files: &TouchedFilePaths,
        projects: &ProjectGraph,
    ) -> Result<Option<NodeIndex>, WorkspaceError> {
        // Validate project first
        let (project_id, task_id) = Target::parse(target)?;
        let project = projects.get(&project_id)?;

        if !project.is_affected(touched_files) {
            trace!(
                target: "moon:dep-graph",
                "Project {} not affected based on touched files, skipping",
                color::id(&project_id),
            );

            return Ok(None);
        }

        // Validate task exists for project
        let task = project.get_task(&task_id)?;

        if !task.is_affected(touched_files)? {
            trace!(
                target: "moon:dep-graph",
                "Project {} task {} not affected based on touched files, skipping",
                color::id(&project_id),
                color::id(&task_id),
            );

            return Ok(None);
        }

        Ok(Some(self.run_target(target, projects)?))
    }

    pub fn to_dot(&self) -> String {
        let dot = Dot::with_config(&self.graph, &[Config::EdgeNoLabel, Config::NodeIndexLabel]);

        // format!("{:?}", dot)
        String::from("")
    }

    fn sync_project(
        &mut self,
        project_id: &str,
        projects: &ProjectGraph,
    ) -> Result<NodeIndex, WorkspaceError> {
        if self.index_cache.contains_key(project_id) {
            return Ok(*self.index_cache.get(project_id).unwrap());
        }

        trace!(
            target: "moon:dep-graph",
            "Syncing project {} configs and dependencies",
            color::id(project_id),
        );

        let project = projects.get(project_id)?;

        // Sync can be run in parallel while deps are installing
        let node = self.graph.add_node(Node {
            label: project_id.to_owned(),
            type_of: NodeType::SyncProject(project_id.to_owned()),
        });

        self.graph.add_edge(self.toolchain_node, node, 0);
        self.nodes
            .insert(node, NodeType::SyncProject(project_id.to_owned()));

        // But we need to wait on all dependent nodes
        for dep_id in projects.get_dependencies_of(&project)? {
            let dep_node = self.sync_project(&dep_id, projects)?;
            self.graph.add_edge(dep_node, node, 0);
        }

        // Also cache so we don't sync the same project multiple times
        self.index_cache.insert(project_id.to_owned(), node);

        Ok(node)
    }
}
