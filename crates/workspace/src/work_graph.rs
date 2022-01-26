use crate::errors::WorkspaceError;
use moon_logger::{color, debug, trace};
use moon_project::{ProjectGraph, Target, TouchedFilePaths};
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use petgraph::Graph;
use std::cell::RefCell;
use std::collections::HashMap;

pub use petgraph::graph::NodeIndex;

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

    /// Mapping of IDs to existing node indices.
    index_cache: RefCell<HashMap<String, NodeIndex>>,

    /// Reference node for the "install deps" job.
    install_deps_node: NodeIndex,

    /// Graph of all projects.
    projects: &'a ProjectGraph,
}

impl<'a> WorkGraph<'a> {
    pub fn new(projects: &'a ProjectGraph) -> Self {
        debug!(
            target: "moon:work-graph",
            "Creating work graph",
        );

        let mut graph = Graph::<JobType, u8>::new();

        // Toolchain must be setup first
        let toolchain_node = graph.add_node(JobType::SetupToolchain);

        // Deps can be installed *after* the toolchain exists
        let install_deps_node = graph.add_node(JobType::InstallNodeDeps);
        graph.add_edge(toolchain_node, install_deps_node, 0);

        WorkGraph {
            graph: RefCell::new(graph),
            index_cache: RefCell::new(HashMap::new()),
            install_deps_node,
            projects,
            toolchain_node,
        }
    }

    pub fn sort_topological(&self) -> Result<Vec<NodeIndex>, WorkspaceError> {
        let graph = self.graph.borrow();

        match toposort(&*graph, None) {
            Ok(nodes) => Ok(nodes),
            Err(error) => Err(WorkspaceError::CycleDetected(error.node_id().index())),
        }
    }

    pub fn run_target(&self, target: &str) -> Result<NodeIndex, WorkspaceError> {
        let mut graph = self.graph.borrow_mut();
        let mut index_cache = self.index_cache.borrow_mut();

        self.do_run_target(target, &mut graph, &mut index_cache)
    }

    pub fn run_target_if_touched(
        &self,
        target: &str,
        touched_files: &TouchedFilePaths,
    ) -> Result<Option<NodeIndex>, WorkspaceError> {
        let mut graph = self.graph.borrow_mut();
        let mut index_cache = self.index_cache.borrow_mut();

        // Validate project first
        let (project_id, task_id) = Target::parse(target)?;
        let project = self.projects.get(&project_id)?;

        if !project.is_affected(touched_files) {
            trace!(
                target: "moon:work-graph",
                "Project {} not affected based on touched files, skipping",
                color::id(&project_id),
            );

            return Ok(None);
        }

        // Validate task exists for project
        let task = project.get_task(&task_id)?;

        if !task.is_affected(touched_files)? {
            trace!(
                target: "moon:work-graph",
                "Project {} task {} not affected based on touched files, skipping",
                color::id(&project_id),
                color::id(&task_id),
            );

            return Ok(None);
        }

        Ok(Some(self.do_run_target(
            target,
            &mut graph,
            &mut index_cache,
        )?))
    }

    pub fn sync_project(&self, project_id: &str) -> Result<NodeIndex, WorkspaceError> {
        let mut graph = self.graph.borrow_mut();
        let mut index_cache = self.index_cache.borrow_mut();

        self.do_sync_project(project_id, &mut graph, &mut index_cache)
    }

    fn do_run_target(
        &self,
        target: &str,
        graph: &mut GraphType,
        index_cache: &mut HashMap<String, NodeIndex>,
    ) -> Result<NodeIndex, WorkspaceError> {
        if index_cache.contains_key(target) {
            return Ok(*index_cache.get(target).unwrap());
        }

        trace!(
            target: "moon:work-graph",
            "Target {} does not exist in the work graph, inserting",
            color::id(target),
        );

        let (project_id, task_id) = Target::parse(target)?;
        let project = self.projects.get(&project_id)?;

        let node = graph.add_node(JobType::RunTarget(target.to_owned()));
        graph.add_edge(self.install_deps_node, node, 0);

        // We should sync projects *before* running targets
        let project_node = self.do_sync_project(&project_id, graph, index_cache)?;
        graph.add_edge(project_node, node, 0);

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
                target: "moon:work-graph",
                "Adding dependencies {} from target {}",
                dep_names.join(", "),
                color::id(target),
            );

            for dep_target in &task.deps {
                let dep_node = self.do_run_target(dep_target, graph, index_cache)?;
                graph.add_edge(dep_node, node, 0);
            }
        }

        // Also cache so we don't run the same target multiple times
        index_cache.insert(target.to_owned(), node);

        Ok(node)
    }

    fn do_sync_project(
        &self,
        project_id: &str,
        graph: &mut GraphType,
        index_cache: &mut HashMap<String, NodeIndex>,
    ) -> Result<NodeIndex, WorkspaceError> {
        if index_cache.contains_key(project_id) {
            return Ok(*index_cache.get(project_id).unwrap());
        }

        trace!(
            target: "moon:work-graph",
            "Syncing project {} configs and dependencies",
            color::id(project_id),
        );

        let project = self.projects.get(project_id)?;
        let node = graph.add_node(JobType::SyncProject(project_id.to_owned()));

        // Sync can be run in parallel while deps are installing
        graph.add_edge(self.toolchain_node, node, 0);

        // But we need to wait on all dependent nodes
        for dep_id in self.projects.get_dependencies_of(&project)? {
            let dep_node = self.do_sync_project(&dep_id, graph, index_cache)?;
            graph.add_edge(dep_node, node, 0);
        }

        // Also cache so we don't sync the same project multiple times
        index_cache.insert(project_id.to_owned(), node);

        Ok(node)
    }
}
