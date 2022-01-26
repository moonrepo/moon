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

type GraphType = DiGraph<Node, ()>;

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
}

impl DepGraph {
    pub fn default() -> Self {
        debug!(
            target: "moon:dep-graph",
            "Creating work graph",
        );

        let mut graph: GraphType = Graph::new();

        // Toolchain must be setup first
        let toolchain_node = graph.add_node(Node {
            label: String::from("SetupToolchain"),
            type_of: NodeType::SetupToolchain,
        });

        // Deps can be installed *after* the toolchain exists
        let install_deps_node = graph.add_node(Node {
            label: String::from("InstallNodeDeps"),
            type_of: NodeType::InstallNodeDeps,
        });

        graph.add_edge(install_deps_node, toolchain_node, ());

        DepGraph {
            graph,
            index_cache: HashMap::new(),
            install_deps_node,
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
            label: format!("RunTarget({})", target),
            type_of: NodeType::RunTarget(target.to_owned()),
        });
        self.graph.add_edge(node, self.install_deps_node, ());

        // We should sync projects *before* running targets
        let project_node = self.sync_project(&project.id, projects)?;
        self.graph.add_edge(node, project_node, ());

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
                self.graph.add_edge(node, dep_node, ());
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

    pub fn sync_project(
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
        let node_index = self.graph.add_node(Node {
            label: format!("SyncProject({})", project_id),
            type_of: NodeType::SyncProject(project_id.to_owned()),
        });

        self.graph.add_edge(node_index, self.toolchain_node, ());

        // But we need to wait on all dependent nodes
        for dep_id in projects.get_dependencies_of(&project)? {
            let dep_node_index = self.sync_project(&dep_id, projects)?;
            self.graph.add_edge(node_index, dep_node_index, ());
        }

        // Also cache so we don't sync the same project multiple times
        self.index_cache.insert(project_id.to_owned(), node_index);

        Ok(node_index)
    }

    pub fn to_dot(&self) -> String {
        let graph = self.graph.map(|_, n| n.label.as_str(), |_, e| e);
        let dot = Dot::with_config(&graph, &[Config::EdgeNoLabel]);

        format!("{:?}", dot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use moon_config::GlobalProjectConfig;
    use moon_project::ProjectGraph;
    use moon_utils::test::get_fixtures_dir;
    use std::collections::HashMap;

    fn create_project_graph() -> ProjectGraph {
        ProjectGraph::new(
            &get_fixtures_dir("projects"),
            GlobalProjectConfig::default(),
            &HashMap::from([
                ("advanced".to_owned(), "advanced".to_owned()),
                ("basic".to_owned(), "basic".to_owned()),
                ("emptyConfig".to_owned(), "empty-config".to_owned()),
                ("noConfig".to_owned(), "no-config".to_owned()),
                ("foo".to_owned(), "deps/foo".to_owned()),
                ("bar".to_owned(), "deps/bar".to_owned()),
                ("baz".to_owned(), "deps/baz".to_owned()),
                ("tasks".to_owned(), "tasks".to_owned()),
            ]),
        )
    }

    #[test]
    fn default_graph() {
        let graph = DepGraph::default();

        assert_snapshot!(graph.to_dot());
    }

    mod run_target {
        use super::*;

        #[test]
        fn single_targets() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph.sync_project("advanced", &projects).unwrap();
            graph.sync_project("basic", &projects).unwrap();
            graph.sync_project("emptyConfig", &projects).unwrap();
            graph.sync_project("noConfig", &projects).unwrap();

            assert_snapshot!(graph.to_dot());
        }
    }

    mod sync_project {
        use super::*;

        #[test]
        fn isolated_projects() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph.sync_project("advanced", &projects).unwrap();
            graph.sync_project("basic", &projects).unwrap();
            graph.sync_project("emptyConfig", &projects).unwrap();
            graph.sync_project("noConfig", &projects).unwrap();

            assert_snapshot!(graph.to_dot());
        }

        #[test]
        fn projects_with_deps() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph.sync_project("foo", &projects).unwrap();
            graph.sync_project("bar", &projects).unwrap();
            graph.sync_project("baz", &projects).unwrap();
            graph.sync_project("basic", &projects).unwrap();

            assert_snapshot!(graph.to_dot());
        }

        #[test]
        fn projects_with_tasks() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph.sync_project("noConfig", &projects).unwrap();
            graph.sync_project("tasks", &projects).unwrap();

            assert_snapshot!(graph.to_dot());
        }
    }
}
