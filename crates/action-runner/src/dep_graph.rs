use crate::errors::DepGraphError;
use crate::node::Node;
use moon_config::ProjectLanguage;
use moon_lang::SupportedLanguage;
use moon_logger::{color, debug, map_list, trace};
use moon_project::{Project, ProjectGraph, Target, TargetError, TargetProject, TouchedFilePaths};
use petgraph::algo::toposort;
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use petgraph::visit::EdgeRef;
use petgraph::Graph;
use std::collections::{HashMap, HashSet};

pub use petgraph::graph::NodeIndex;

const TARGET: &str = "moon:dep-graph";

fn get_lang_from_project(project: &Project) -> SupportedLanguage {
    match &project.config.language {
        ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => SupportedLanguage::Node,
        _ => SupportedLanguage::System,
    }
}

pub type DepGraphType = DiGraph<Node, ()>;
pub type BatchedTopoSort = Vec<Vec<NodeIndex>>;

/// A directed acyclic graph (DAG) for the work that needs to be processed, based on a
/// project or task's dependency chain. This is also known as a "task graph" (not to
/// be confused with ours) or a "dependency graph".
pub struct DepGraph {
    pub graph: DepGraphType,

    indices: HashMap<Node, NodeIndex>,
}

impl DepGraph {
    pub fn default() -> Self {
        debug!(target: TARGET, "Creating dependency graph",);

        let mut graph: DepGraphType = Graph::new();
        let setup_toolchain_index = graph.add_node(Node::SetupToolchain);

        DepGraph {
            graph,
            indices: HashMap::from([(Node::SetupToolchain, setup_toolchain_index)]),
        }
    }

    pub fn get_index_from_node(&self, node: &Node) -> Option<&NodeIndex> {
        self.indices.get(node)
    }

    pub fn get_node_from_index(&self, index: &NodeIndex) -> Option<&Node> {
        self.graph.node_weight(*index)
    }

    pub fn get_or_insert_node(&mut self, node: Node) -> NodeIndex {
        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        let index = self.graph.add_node(node.clone());

        self.indices.insert(node, index);

        index
    }

    pub fn install_deps(&mut self, lang: SupportedLanguage) -> NodeIndex {
        let node = Node::InstallDeps(lang.clone());

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        trace!(target: TARGET, "Installing {} dependencies", lang.label());

        let setup_toolchain_index = self.get_or_insert_node(Node::SetupToolchain);
        let install_deps_index = self.get_or_insert_node(node);

        self.graph
            .add_edge(install_deps_index, setup_toolchain_index, ());

        install_deps_index
    }

    pub fn install_project_deps(
        &mut self,
        project_id: &str,
        projects: &ProjectGraph,
    ) -> Result<NodeIndex, DepGraphError> {
        let project = projects.load(project_id)?;
        let lang = get_lang_from_project(&project);

        Ok(self.install_deps(lang))
    }

    pub fn run_target(
        &mut self,
        target: &Target,
        projects: &ProjectGraph,
        touched_files: Option<&TouchedFilePaths>,
    ) -> Result<usize, DepGraphError> {
        let task_id = &target.task_id;
        let mut inserted_count = 0;

        match &target.project {
            // :task
            TargetProject::All => {
                for project_id in projects.ids() {
                    let project = projects.load(&project_id)?;

                    if project.tasks.contains_key(task_id)
                        && self
                            .insert_target(&project_id, task_id, projects, touched_files)?
                            .is_some()
                    {
                        inserted_count += 1;
                    }
                }
            }
            // ^:task
            TargetProject::Deps => {
                target.fail_with(TargetError::NoProjectDepsInRunContext)?;
            }
            // project:task
            TargetProject::Id(project_id) => {
                if self
                    .insert_target(project_id, task_id, projects, touched_files)?
                    .is_some()
                {
                    inserted_count += 1;
                }
            }
            // ~:task
            TargetProject::Own => {
                target.fail_with(TargetError::NoProjectSelfInRunContext)?;
            }
        };

        Ok(inserted_count)
    }

    pub fn run_target_dependents(
        &mut self,
        target: &Target,
        projects: &ProjectGraph,
    ) -> Result<(), DepGraphError> {
        trace!(
            target: TARGET,
            "Adding dependents to run for target {}",
            color::target(&target.id),
        );

        let (project_id, task_id) = target.ids()?;
        let project = projects.load(&project_id)?;
        let dependents = projects.get_dependents_of(&project)?;

        for dependent_id in dependents {
            let dependent = projects.load(&dependent_id)?;

            if dependent.tasks.contains_key(&task_id) {
                self.run_target(&Target::new(&dependent_id, &task_id)?, projects, None)?;
            }
        }

        Ok(())
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
        project_id: &str,
        projects: &ProjectGraph,
    ) -> Result<NodeIndex, DepGraphError> {
        let project = projects.load(project_id)?;
        let lang = get_lang_from_project(&project);
        let node = Node::SyncProject(lang, project_id.to_owned());

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(*index);
        }

        trace!(
            target: TARGET,
            "Syncing project {} configs and dependencies",
            color::id(project_id),
        );

        // Sync can be run in parallel while deps are installing
        let setup_toolchain_index = self.get_or_insert_node(Node::SetupToolchain);
        let sync_project_index = self.get_or_insert_node(node);

        self.graph
            .add_edge(sync_project_index, setup_toolchain_index, ());

        // But we need to wait on all dependent nodes
        for dep_id in projects.get_dependencies_of(&project)? {
            let sync_dep_project_index = self.sync_project(&dep_id, projects)?;

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

                if id == &Node::SetupToolchain.label() {
                    format!(
                        "label=\"{}\" style=filled, shape=oval, fillcolor=black, fontcolor=white",
                        id
                    )
                } else {
                    format!(
                        "label=\"{}\" style=filled, shape=oval, fillcolor=gray, fontcolor=black",
                        id
                    )
                }
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
        project_id: &str,
        task_id: &str,
        projects: &ProjectGraph,
        touched_files: Option<&TouchedFilePaths>,
    ) -> Result<Option<NodeIndex>, DepGraphError> {
        let target_id = Target::format(project_id, task_id)?;
        let node = Node::RunTarget(target_id.clone());

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(*index));
        }

        let project = projects.load(project_id)?;

        // Compare against touched files if provided
        if let Some(touched) = touched_files {
            if !project.get_task(task_id)?.is_affected(touched)? {
                trace!(
                    target: TARGET,
                    "Project {} task {} not affected based on touched files, skipping",
                    color::id(project_id),
                    color::id(task_id),
                );

                return Ok(None);
            }
        }

        trace!(
            target: TARGET,
            "Target {} does not exist in the dependency graph, inserting",
            color::target(&target_id),
        );

        // We should install deps & sync projects *before* running targets
        let install_deps_index = self.install_project_deps(&project.id, projects)?;
        let sync_project_index = self.sync_project(&project.id, projects)?;
        let run_target_index = self.get_or_insert_node(node);

        self.graph
            .add_edge(run_target_index, install_deps_index, ());
        self.graph
            .add_edge(run_target_index, sync_project_index, ());

        // And we also need to wait on all dependent nodes
        let task = project.get_task(task_id)?;

        if !task.deps.is_empty() {
            trace!(
                target: TARGET,
                "Adding dependencies {} from target {}",
                map_list(&task.deps, |f| color::symbol(f)),
                color::target(&target_id),
            );

            for dep_target_id in &task.deps {
                let dep_target = Target::parse(dep_target_id)?;

                if let Some(run_dep_target_index) = self.insert_target(
                    &dep_target.project_id.unwrap(),
                    &dep_target.task_id,
                    projects,
                    touched_files,
                )? {
                    self.graph
                        .add_edge(run_target_index, run_dep_target_index, ());
                }
            }
        }

        Ok(Some(run_target_index))
    }
}
