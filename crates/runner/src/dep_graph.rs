use crate::errors::DepGraphError;
use crate::node::ActionNode;
use moon_config::{default_node_version, ProjectLanguage, ProjectWorkspaceNodeConfig};
use moon_contract::Runtime;
use moon_logger::{color, debug, map_list, trace};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_task::{Target, TargetError, TargetProjectScope, TouchedFilePaths};
use petgraph::algo::toposort;
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use petgraph::visit::EdgeRef;
use petgraph::Graph;
use std::collections::{HashMap, HashSet};

pub use petgraph::graph::NodeIndex;

const LOG_TARGET: &str = "moon:dep-graph";

pub type DepGraphType = DiGraph<ActionNode, ()>;
pub type BatchedTopoSort = Vec<Vec<NodeIndex>>;

/// A directed acyclic graph (DAG) for the work that needs to be processed, based on a
/// project or task's dependency chain. This is also known as a "task graph" (not to
/// be confused with ours) or a "dependency graph".
pub struct DepGraph {
    pub graph: DepGraphType,

    indices: HashMap<ActionNode, NodeIndex>,
}

impl DepGraph {
    pub fn default() -> Self {
        debug!(target: LOG_TARGET, "Creating dependency graph",);

        DepGraph {
            graph: Graph::new(),
            indices: HashMap::new(),
        }
    }

    pub fn get_index_from_node(&self, node: &ActionNode) -> Option<&NodeIndex> {
        self.indices.get(node)
    }

    pub fn get_node_from_index(&self, index: &NodeIndex) -> Option<&ActionNode> {
        self.graph.node_weight(*index)
    }

    pub fn get_or_insert_node(&mut self, node: ActionNode) -> NodeIndex {
        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        let index = self.graph.add_node(node.clone());

        self.indices.insert(node, index);

        index
    }

    #[track_caller]
    pub fn get_runtime_from_project(
        &self,
        project: &Project,
        project_graph: &ProjectGraph,
    ) -> Runtime {
        match &project.config.language {
            ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => {
                let version = match &project.config.workspace.node {
                    Some(ProjectWorkspaceNodeConfig {
                        version: Some(version),
                        ..
                    }) => version.to_owned(),
                    _ => match &project_graph.workspace_config.node {
                        Some(node) => node.version.to_owned(),
                        None => default_node_version(),
                    },
                };

                Runtime::Node(version)
            }
            _ => Runtime::System,
        }
    }

    pub fn install_deps(&mut self, runtime: &Runtime) -> Result<NodeIndex, DepGraphError> {
        let node = ActionNode::InstallDeps(runtime.clone());

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(*index);
        }

        trace!(
            target: LOG_TARGET,
            "Installing {} dependencies",
            runtime.label()
        );

        let setup_toolchain_index = self.setup_tool(runtime);
        let install_deps_index = self.get_or_insert_node(node);

        self.graph
            .add_edge(install_deps_index, setup_toolchain_index, ());

        Ok(install_deps_index)
    }

    pub fn install_project_deps(
        &mut self,
        runtime: &Runtime,
        project: &Project,
        project_graph: &ProjectGraph,
    ) -> Result<NodeIndex, DepGraphError> {
        let mut node = ActionNode::InstallDeps(runtime.clone());

        for platform_service in &project_graph.platforms {
            if platform_service.is(runtime) {
                // If project is not in the package manager workspace,
                // update the node to install deps into the project directly!
                if !platform_service.is_project_in_package_manager_workspace(
                    &project.id,
                    &project.root,
                    &project_graph.workspace_root,
                    &project_graph.workspace_config,
                )? {
                    node = ActionNode::InstallProjectDeps(runtime.clone(), project.id.clone())
                }

                break;
            }
        }

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(*index);
        }

        trace!(
            target: LOG_TARGET,
            "Installing {} dependencies",
            runtime.label()
        );

        let setup_toolchain_index = self.setup_tool(runtime);
        let install_deps_index = self.get_or_insert_node(node);

        self.graph
            .add_edge(install_deps_index, setup_toolchain_index, ());

        Ok(install_deps_index)
    }

    pub fn run_target<T: AsRef<Target>>(
        &mut self,
        target: T,
        project_graph: &ProjectGraph,
        touched_files: &Option<TouchedFilePaths>,
    ) -> Result<usize, DepGraphError> {
        let target = target.as_ref();
        let task_id = &target.task_id;
        let mut inserted_count = 0;

        match &target.project {
            // :task
            TargetProjectScope::All => {
                for project_id in project_graph.ids() {
                    let project = project_graph.load(&project_id)?;

                    if project.tasks.contains_key(task_id)
                        && self
                            .insert_target(task_id, &project, project_graph, touched_files)?
                            .is_some()
                    {
                        inserted_count += 1;
                    }
                }
            }
            // ^:task
            TargetProjectScope::Deps => {
                target.fail_with(TargetError::NoProjectDepsInRunContext)?;
            }
            // project:task
            TargetProjectScope::Id(project_id) => {
                let project = project_graph.load(project_id)?;

                if self
                    .insert_target(task_id, &project, project_graph, touched_files)?
                    .is_some()
                {
                    inserted_count += 1;
                }
            }
            // ~:task
            TargetProjectScope::OwnSelf => {
                target.fail_with(TargetError::NoProjectSelfInRunContext)?;
            }
        };

        Ok(inserted_count)
    }

    pub fn run_target_dependents<T: AsRef<Target>>(
        &mut self,
        target: T,
        project_graph: &ProjectGraph,
    ) -> Result<(), DepGraphError> {
        let target = target.as_ref();

        trace!(
            target: LOG_TARGET,
            "Adding dependents to run for target {}",
            color::target(&target.id),
        );

        let (project_id, task_id) = target.ids()?;
        let project = project_graph.load(&project_id)?;
        let dependents = project_graph.get_dependents_of(&project)?;

        for dependent_id in dependents {
            let dependent = project_graph.load(&dependent_id)?;

            if dependent.tasks.contains_key(&task_id) {
                self.run_target(Target::new(&dependent_id, &task_id)?, project_graph, &None)?;
            }
        }

        Ok(())
    }

    pub fn run_targets_by_id(
        &mut self,
        target_ids: &[String],
        project_graph: &ProjectGraph,
        touched_files: &Option<TouchedFilePaths>,
    ) -> Result<(Vec<String>, usize), DepGraphError> {
        let mut qualified_targets = vec![];
        let mut inserted_count = 0;

        for target_id in target_ids {
            let target = Target::parse(target_id)?;

            // Extract the fully qualified target name from the task itself.
            // We do this to resolve any project aliases being used.
            if let Some(project_id) = &target.project_id {
                qualified_targets.push(
                    project_graph
                        .load(project_id)?
                        .get_task(&target.task_id)?
                        .target
                        .clone(),
                );
            }

            // Keep track of how many transitive targets were inserted!
            inserted_count += self.run_target(target, project_graph, touched_files)?;
        }

        Ok((qualified_targets, inserted_count))
    }

    pub fn setup_tool(&mut self, runtime: &Runtime) -> NodeIndex {
        let node = ActionNode::SetupTool(runtime.clone());

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        trace!(target: LOG_TARGET, "Setting up {} tool", runtime.label());

        self.get_or_insert_node(node)
    }

    pub fn sort_topological(&self) -> Result<Vec<NodeIndex>, DepGraphError> {
        let list = match toposort(&self.graph, None) {
            Ok(nodes) => nodes,
            Err(error) => {
                return Err(DepGraphError::CycleDetected(
                    self.get_node_from_index(&error.node_id()).unwrap().label(),
                ));
            }
        };

        Ok(list.into_iter().rev().collect())
    }

    pub fn sort_batched_topological(&self) -> Result<BatchedTopoSort, DepGraphError> {
        let mut batches: BatchedTopoSort = vec![];

        // Count how many times an index is referenced across nodes and edges
        let mut node_counts = HashMap::<NodeIndex, u32>::new();

        for ix in self.graph.node_indices() {
            node_counts.entry(ix).and_modify(|e| *e += 1).or_insert(0);

            for dep_ix in self.graph.neighbors(ix) {
                node_counts
                    .entry(dep_ix)
                    .and_modify(|e| *e += 1)
                    .or_insert(0);
            }
        }

        // Gather root nodes (count of 0)
        let mut root_nodes = HashSet::<NodeIndex>::new();

        for (ix, count) in &node_counts {
            if *count == 0 {
                root_nodes.insert(*ix);
            }
        }

        // If no root nodes are found, but nodes exist, then we have a cycle
        if root_nodes.is_empty() && !node_counts.is_empty() {
            self.detect_cycle()?;
        }

        while !root_nodes.is_empty() {
            // Push this batch onto the list
            batches.push(root_nodes.clone().into_iter().collect());

            // Reset the root nodes and find new ones after decrementing
            let mut next_root_nodes = HashSet::<NodeIndex>::new();

            for ix in &root_nodes {
                for dep_ix in self.graph.neighbors(*ix) {
                    let count = node_counts
                        .entry(dep_ix)
                        .and_modify(|e| *e -= 1)
                        .or_insert(0);

                    if *count == 0 {
                        next_root_nodes.insert(dep_ix);
                    }
                }
            }

            root_nodes = next_root_nodes;
        }

        Ok(batches.into_iter().rev().collect())
    }

    pub fn sync_project(
        &mut self,
        runtime: &Runtime,
        project: &Project,
        project_graph: &ProjectGraph,
    ) -> Result<NodeIndex, DepGraphError> {
        let node = ActionNode::SyncProject(runtime.clone(), project.id.clone());

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(*index);
        }

        trace!(
            target: LOG_TARGET,
            "Syncing project {} configs and dependencies",
            color::id(&project.id),
        );

        // Sync can be run in parallel while deps are installing
        let setup_toolchain_index = self.setup_tool(runtime);
        let sync_project_index = self.get_or_insert_node(node);

        self.graph
            .add_edge(sync_project_index, setup_toolchain_index, ());

        // But we need to wait on all dependent nodes
        for dep_id in project_graph.get_dependencies_of(project)? {
            let dep_project = project_graph.load(&dep_id)?;
            let dep_runtime = self.get_runtime_from_project(&dep_project, project_graph);

            let sync_dep_project_index =
                self.sync_project(&dep_runtime, &dep_project, project_graph)?;

            self.graph
                .add_edge(sync_project_index, sync_dep_project_index, ());
        }

        Ok(sync_project_index)
    }

    pub fn to_dot(&self) -> String {
        let graph = self.graph.map(|_, n| n.label(), |_, e| e);

        let dot = Dot::with_attr_getters(
            &graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, e| {
                if e.source().index() == 0 {
                    String::from("arrowhead=none")
                } else {
                    String::from("arrowhead=box, arrowtail=box")
                }
            },
            &|_, n| {
                let id = n.1;

                format!(
                    "label=\"{}\" style=filled, shape=oval, fillcolor=gray, fontcolor=black",
                    id
                )
            },
        );

        format!("{:?}", dot)
    }

    #[track_caller]
    fn detect_cycle(&self) -> Result<(), DepGraphError> {
        use petgraph::algo::kosaraju_scc;

        let scc = kosaraju_scc(&self.graph);
        let cycle = scc
            .last()
            .unwrap()
            .iter()
            .map(|i| self.get_node_from_index(i).unwrap().label())
            .collect::<Vec<String>>()
            .join(" → ");

        Err(DepGraphError::CycleDetected(cycle))
    }

    fn insert_target(
        &mut self,
        task_id: &str,
        project: &Project,
        project_graph: &ProjectGraph,
        touched_files: &Option<TouchedFilePaths>,
    ) -> Result<Option<NodeIndex>, DepGraphError> {
        let target_id = Target::format(&project.id, task_id)?;
        let node = ActionNode::RunTarget(target_id.clone());

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(*index));
        }

        // Compare against touched files if provided
        if let Some(touched) = touched_files {
            if !project.get_task(task_id)?.is_affected(touched)? {
                trace!(
                    target: LOG_TARGET,
                    "Project {} task {} not affected based on touched files, skipping",
                    color::id(&project.id),
                    color::id(task_id),
                );

                return Ok(None);
            }
        }

        trace!(
            target: LOG_TARGET,
            "Target {} does not exist in the dependency graph, inserting",
            color::target(&target_id),
        );

        // We should install deps & sync projects *before* running targets
        let runtime = self.get_runtime_from_project(project, project_graph);
        let install_deps_index = self.install_project_deps(&runtime, project, project_graph)?;
        let sync_project_index = self.sync_project(&runtime, project, project_graph)?;
        let run_target_index = self.get_or_insert_node(node);

        self.graph
            .add_edge(run_target_index, install_deps_index, ());
        self.graph
            .add_edge(run_target_index, sync_project_index, ());

        // And we also need to wait on all dependent nodes
        let task = project.get_task(task_id)?;

        if !task.deps.is_empty() {
            let parallel = task.options.run_deps_in_parallel;
            let mut previous_target_index = None;

            trace!(
                target: LOG_TARGET,
                "Adding dependencies {} from target {}",
                map_list(&task.deps, |f| color::symbol(f)),
                color::target(&target_id),
            );

            for dep_target_id in &task.deps {
                let dep_target = Target::parse(dep_target_id)?;
                let dep_project = project_graph.load(&dep_target.project_id.unwrap())?;

                if let Some(run_dep_target_index) = self.insert_target(
                    &dep_target.task_id,
                    &dep_project,
                    project_graph,
                    touched_files,
                )? {
                    // When parallel, parent depends on child
                    if parallel {
                        self.graph
                            .add_edge(run_target_index, run_dep_target_index, ());

                        // When serial, next child depends on previous child
                    } else if let Some(prev) = previous_target_index {
                        self.graph.add_edge(run_dep_target_index, prev, ());
                    }

                    previous_target_index = Some(run_dep_target_index);
                }
            }

            if !parallel {
                self.graph
                    .add_edge(run_target_index, previous_target_index.unwrap(), ());
            }
        }

        Ok(Some(run_target_index))
    }
}
