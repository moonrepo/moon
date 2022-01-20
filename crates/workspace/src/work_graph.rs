use crate::errors::WorkspaceError;
use moon_project::{ProjectGraph, Target, TaskGraph};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Graph;
use petgraph::algo::toposort;
use std::cell::RefCell;
use std::collections::HashMap;

pub enum JobType {
    InstallNodeDeps,
    RunTarget(String), // target id
    SetupToolchain,
    SyncProject(String), // project id
}

type GraphType = DiGraph<JobType, u8>;

/// A directed acyclic graph (DAG) for the work that needs to be processed, based on a
/// project or task's dependency chain. This is also known as a "task graph" (not to
/// be confused with ours) or a "dependency graph". We call it a "work graph", as it's
/// the combination of those 2 with additional information for the work unit (a job).
pub struct WorkGraph<'a> {
    graph: RefCell<GraphType>,

    /// Reference node for the "setup toolchain" job.
    toolchain_node: NodeIndex,

    /// Reference node for the "install deps" job.
    install_deps_node: NodeIndex,

    /// Graph of all projects.
    projects: &'a ProjectGraph,

    /// Mapping of project IDs to existing node indices.
    synced_projects: RefCell<HashMap<String, NodeIndex>>,

    /// Graph of all tasks indexed by targets.
    tasks: &'a TaskGraph,
}

impl<'a> WorkGraph<'a> {
    pub fn new(projects: &'a ProjectGraph, tasks: &'a TaskGraph) -> Self {
        let mut graph = Graph::<JobType, u8>::new();

        // Toolchain must be setup first
        let toolchain_node = graph.add_node(JobType::SetupToolchain);

        // Deps can be installed *after* the toolchain exists
        let install_deps_node = graph.add_node(JobType::InstallNodeDeps);
        graph.add_edge(toolchain_node, install_deps_node, 0);

        WorkGraph {
            graph: RefCell::new(graph),
            toolchain_node,
            install_deps_node,
            projects,
            synced_projects: RefCell::new(HashMap::new()),
            tasks,
        }
    }

    pub fn sort_topological(&self) -> Result<Vec<NodeIndex>, WorkspaceError> {
        let graph = self.graph.borrow();

        match toposort(&*graph, None) {
            Ok(nodes) => Ok(nodes),
            Err(error) => Err(WorkspaceError::CycleDetected(error.node_id().index()))
        }
    }

    pub fn run_target(&self, target: &str) -> Result<NodeIndex, WorkspaceError> {
        let mut graph = self.graph.borrow_mut();
        let mut synced_projects = self.synced_projects.borrow_mut();

        self.do_run_target(target, &mut graph, &mut synced_projects)
    }

    pub fn sync_project(&self, project_id: &str) -> Result<NodeIndex, WorkspaceError> {
        let mut graph = self.graph.borrow_mut();
        let mut synced_projects = self.synced_projects.borrow_mut();

        self.do_sync_project(project_id, &mut graph, &mut synced_projects)
    }

    fn do_run_target(
        &self,
        target: &str,
        graph: &mut GraphType,
        synced_projects: &mut HashMap<String, NodeIndex>,
    ) -> Result<NodeIndex, WorkspaceError> {
        let (project_id, _) = Target::parse(target)?;

        let node = graph.add_node(JobType::RunTarget(target.to_owned()));
        graph.add_edge(self.install_deps_node, node, 0);

        // We should sync projects *before* running targets
        let project_node = self.do_sync_project(&project_id, graph, synced_projects)?;
        graph.add_edge(project_node, node, 0);

        // And we also need to wait on all dependent nodes
        let task = self.tasks.get(target).unwrap();

        if !task.deps.is_empty() {
            for dep_target in &task.deps {
                let dep_node = self.do_run_target(dep_target, graph, synced_projects)?;
                graph.add_edge(dep_node, node, 0);
            }
        }

        Ok(node)
    }

    fn do_sync_project(
        &self,
        project_id: &str,
        graph: &mut GraphType,
        synced_projects: &mut HashMap<String, NodeIndex>,
    ) -> Result<NodeIndex, WorkspaceError> {
        if synced_projects.contains_key(project_id) {
            return Ok(*synced_projects.get(project_id).unwrap());
        }

        let project = self.projects.get(project_id)?;
        let node = graph.add_node(JobType::SyncProject(project_id.to_owned()));

        // Sync can be run in parallel while deps are installing
        graph.add_edge(self.toolchain_node, node, 0);

        // But we need to wait on all dependent nodes
        for dep_id in self.projects.get_dependencies_of(&project)? {
            let dep_node = self.do_sync_project(&dep_id, graph, synced_projects)?;
            graph.add_edge(dep_node, node, 0);
        }

        // Also cache so we don't sync the same project multiple times
        synced_projects.insert(project_id.to_owned(), node);

        Ok(node)
    }
}
