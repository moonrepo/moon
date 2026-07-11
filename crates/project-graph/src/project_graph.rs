use crate::project_graph_error::ProjectGraphError;
use daggy::Dag;
use moon_common::Id;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_config::DependencyScope;
use moon_graph_utils::*;
use moon_project::Project;
use moon_project_expander::{ProjectExpander, ProjectExpanderContext};
use petgraph::Direction;
use petgraph::algo::{has_path_connecting, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{EdgeFiltered, EdgeRef};
use rustc_hash::{FxHashMap, FxHashSet};
use scc::hash_map::Entry;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, instrument};

/// The internal graph partition that a dependency scope belongs to. Each
/// partition is individually acyclic, while their union may contain cycles
/// that cross the partition boundary. Anything that requires an acyclic
/// graph (ordering, unguarded recursion) should operate on a partition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopePartition {
    /// Production and peer scoped dependencies.
    Production,

    /// Development, build, and root scoped dependencies.
    Development,
}

impl ScopePartition {
    /// Return the partition that the provided scope belongs to.
    pub fn of(scope: &DependencyScope) -> ScopePartition {
        if is_production_scope(scope) {
            ScopePartition::Production
        } else {
            ScopePartition::Development
        }
    }
}

/// Return true if the provided scope belongs to the production partition
/// (production/peer), and false if the development partition
/// (development/build/root).
pub fn is_production_scope(scope: &DependencyScope) -> bool {
    matches!(scope, DependencyScope::Production | DependencyScope::Peer)
}

/// Return true if adding an edge from source to target with the provided scope
/// would introduce a cycle within the partition that the scope belongs to.
/// Edges belonging to the other partition are ignored, so cycles that cross
/// the production/development boundary are allowed.
pub fn would_cycle_in_scope<N>(
    graph: &DiGraph<N, DependencyScope>,
    source: NodeIndex,
    target: NodeIndex,
    scope: &DependencyScope,
) -> bool {
    let partition = is_production_scope(scope);
    let partitioned_graph = EdgeFiltered::from_fn(graph, |edge| {
        is_production_scope(edge.weight()) == partition
    });

    has_path_connecting(&partitioned_graph, target, source, None)
}

#[derive(Clone, Debug)]
pub struct ProjectNode {
    pub index: NodeIndex,
    pub project: Project,
}

#[derive(Debug, Default)]
pub struct ProjectGraph {
    pub context: GraphExpanderContext,

    /// Map of aliases to project IDs.
    pub aliases: FxHashMap<String, Id>,

    /// ID of the default project.
    pub default_id: Option<Id>,

    /// Union graph of projects (by index) and their dependencies across every
    /// scope. Powers all read APIs. May contain cycles that cross the
    /// production/development boundary; each partitioned graph below is
    /// individually acyclic. Populate with [`ProjectGraph::set_graph`].
    graph: DiGraph<NodeIndex, DependencyScope>,

    /// Directed-acyclic graph (DAG) of production and peer dependencies.
    production_graph: Dag<NodeIndex, DependencyScope>,

    /// Directed-acyclic graph (DAG) of development, build, and root dependencies.
    development_graph: Dag<NodeIndex, DependencyScope>,

    /// Map of node indexes to project IDs.
    pub indexes: FxHashMap<NodeIndex, Id>,

    /// Map of project nodes by ID.
    pub nodes: FxHashMap<Id, ProjectNode>,

    /// Cache of file path lookups, mapped by starting path to project ID (as a string).
    fs_cache: Arc<scc::HashMap<PathBuf, Arc<String>>>,

    /// Map of expanded projects by ID.
    projects: Arc<scc::HashMap<Id, Arc<Project>>>,
}

impl ProjectGraph {
    pub fn new(context: GraphExpanderContext) -> Self {
        debug!("Creating project graph");

        Self {
            context,
            ..Default::default()
        }
    }

    /// Return a map of aliases to their project IDs. Projects without aliases are omitted.
    pub fn aliases(&self) -> FxHashMap<&str, &Id> {
        self.aliases
            .iter()
            .map(|(alias, id)| (alias.as_str(), id))
            .collect()
    }

    /// Return a project with the provided ID or alias from the graph.
    /// If the project does not exist or has been misconfigured, return an error.
    #[instrument(name = "get_project", skip(self))]
    pub fn get(&self, id_or_alias: &str) -> miette::Result<Arc<Project>> {
        self.internal_get(id_or_alias)
    }

    /// Return an unexpanded project with the provided ID or alias from the graph.
    pub fn get_unexpanded(&self, id_or_alias: &str) -> miette::Result<&Project> {
        let id = self.resolve_id(id_or_alias);

        let node = self
            .nodes
            .get(&id)
            .ok_or_else(|| ProjectGraphError::UnconfiguredID { id: id.to_string() })?;

        Ok(&node.project)
    }

    /// Return all projects from the graph.
    #[instrument(name = "get_all_projects", skip(self))]
    pub fn get_all(&self) -> miette::Result<Vec<Arc<Project>>> {
        let mut all = vec![];

        for id in self.nodes.keys() {
            all.push(self.internal_get(id)?);
        }

        Ok(all)
    }

    /// Return all unexpanded projects from the graph.
    pub fn get_all_unexpanded(&self) -> Vec<&Project> {
        self.nodes.values().map(|node| &node.project).collect()
    }

    /// Return the default project if it has been configured and exists.
    pub fn get_default(&self) -> miette::Result<Arc<Project>> {
        if let Some(id) = &self.default_id {
            return self.get(id);
        }

        Err(ProjectGraphError::NoDefaultProject.into())
    }

    /// Find and return a project based on the initial path location.
    /// This will attempt to find the closest matching project source.
    #[instrument(name = "get_project_from_path", skip(self))]
    pub fn get_from_path(&self, starting_file: Option<&Path>) -> miette::Result<Arc<Project>> {
        let current_file = starting_file.unwrap_or(&self.context.working_dir);

        let file = if current_file == self.context.workspace_root {
            Path::new(".")
        } else if let Ok(rel_file) = current_file.strip_prefix(&self.context.workspace_root) {
            rel_file
        } else {
            current_file
        };

        let id = self.internal_search(file)?;

        self.get(&id)
    }

    /// Return a map of project IDs to their file source paths.
    pub fn sources(&self) -> FxHashMap<&Id, &WorkspaceRelativePathBuf> {
        self.nodes
            .iter()
            .map(|(id, node)| (id, &node.project.source))
            .collect()
    }

    /// Return the graph of production and peer dependencies.
    pub fn production_graph(&self) -> &DiGraph<NodeIndex, DependencyScope> {
        self.partitioned_graph(ScopePartition::Production)
    }

    /// Return the graph of development, build, and root dependencies.
    pub fn development_graph(&self) -> &DiGraph<NodeIndex, DependencyScope> {
        self.partitioned_graph(ScopePartition::Development)
    }

    /// Return the graph of dependencies for the provided partition. Unlike
    /// the unioned graph, partitioned graphs are guaranteed to be acyclic.
    pub fn partitioned_graph(
        &self,
        partition: ScopePartition,
    ) -> &DiGraph<NodeIndex, DependencyScope> {
        match partition {
            ScopePartition::Production => self.production_graph.graph(),
            ScopePartition::Development => self.development_graph.graph(),
        }
    }

    /// Return a list of direct project IDs that the provided project depends on,
    /// only traversing edges within the provided partition.
    pub fn partitioned_dependencies_of(
        &self,
        project: &Project,
        partition: ScopePartition,
    ) -> Vec<Id> {
        self.partitioned_neighbors_of(project, partition, Direction::Outgoing)
    }

    /// Return a list of direct project IDs that depend on the provided project,
    /// only traversing edges within the provided partition.
    pub fn partitioned_dependents_of(
        &self,
        project: &Project,
        partition: ScopePartition,
    ) -> Vec<Id> {
        self.partitioned_neighbors_of(project, partition, Direction::Incoming)
    }

    /// Return a list of all project IDs that the provided project depends on,
    /// only traversing edges within the provided partition.
    pub fn partitioned_deep_dependencies_of(
        &self,
        project: &Project,
        partition: ScopePartition,
    ) -> Vec<Id> {
        self.partitioned_traverse(project, partition, Direction::Outgoing)
    }

    /// Return a list of all project IDs that depend on the provided project,
    /// only traversing edges within the provided partition.
    pub fn partitioned_deep_dependents_of(
        &self,
        project: &Project,
        partition: ScopePartition,
    ) -> Vec<Id> {
        self.partitioned_traverse(project, partition, Direction::Incoming)
    }

    /// Return all project IDs sorted topologically in dependency-first order
    /// (dependencies before the projects that depend on them), using only
    /// edges within the provided partition. Projects without edges in the
    /// partition are included. Sorting is only possible for partitioned
    /// graphs, as the unioned graph may contain cycles across partitions.
    pub fn partitioned_toposort(&self, partition: ScopePartition) -> Vec<Id> {
        let mut indices = toposort(self.partitioned_graph(partition), None)
            .expect("Partitioned graphs are always acyclic!");

        // Edges point from a project to its dependency,
        // so reverse the order to get dependencies first
        indices.reverse();

        indices
            .into_iter()
            .map(|index| self.indexes[&index].clone())
            .collect()
    }

    fn partitioned_neighbors_of(
        &self,
        project: &Project,
        partition: ScopePartition,
        direction: Direction,
    ) -> Vec<Id> {
        self.partitioned_graph(partition)
            .neighbors_directed(self.nodes[&project.id].index, direction)
            .map(|index| self.indexes[&index].clone())
            .collect()
    }

    fn partitioned_traverse(
        &self,
        project: &Project,
        partition: ScopePartition,
        direction: Direction,
    ) -> Vec<Id> {
        let graph = self.partitioned_graph(partition);
        let start = self.nodes[&project.id].index;
        let mut visited = FxHashSet::from_iter([start]);
        let mut queue = VecDeque::from([start]);
        let mut results = vec![];

        while let Some(index) = queue.pop_front() {
            for next_index in graph.neighbors_directed(index, direction) {
                if visited.insert(next_index) {
                    results.push(self.indexes[&next_index].clone());
                    queue.push_back(next_index);
                }
            }
        }

        results
    }

    /// Set the union graph of all dependency edges, and derive the production
    /// (production/peer) and development (development/build/root) graphs from it,
    /// with each partition enforcing acyclicity for its own edges. Cycles within
    /// a single partition return an error, while cycles that cross the partition
    /// boundary are allowed. The `indexes` map should be populated beforehand,
    /// so that cycle errors can report project IDs.
    pub fn set_graph(
        &mut self,
        mut graph: DiGraph<NodeIndex, DependencyScope>,
    ) -> miette::Result<()> {
        let mut production_graph = Dag::with_capacity(graph.node_count(), graph.edge_count());
        let mut development_graph = Dag::with_capacity(graph.node_count(), graph.edge_count());

        // Weight-based lookups require each node's weight to be its own
        // index, but the builders may provide stale pre-filtered indices
        // when placeholder nodes were dropped, so rewrite them
        for index in 0..graph.node_count() {
            let index = NodeIndex::new(index);
            graph[index] = index;
        }

        // Mirror the nodes into both graphs, in the same order,
        // so that all node indexes align
        for index in graph.node_indices() {
            production_graph.add_node(graph[index]);
            development_graph.add_node(graph[index]);
        }

        // Then route each edge into the graph its scope belongs to,
        // relying on daggy's insertion checks to detect cycles
        for edge in graph.edge_references() {
            let scope = *edge.weight();

            let partitioned_graph = if is_production_scope(&scope) {
                &mut production_graph
            } else {
                &mut development_graph
            };

            partitioned_graph
                .add_edge(edge.source(), edge.target(), scope)
                .map_err(|_| ProjectGraphError::WouldCycle {
                    source_id: self.label_index(edge.source()),
                    target_id: self.label_index(edge.target()),
                })?;
        }

        self.graph = graph;
        self.production_graph = production_graph;
        self.development_graph = development_graph;

        Ok(())
    }

    /// Focus the graph for a specific project by ID.
    pub fn focus_for(&self, id_or_alias: &Id, with_dependents: bool) -> miette::Result<Self> {
        let project = self.get(id_or_alias)?;
        let focused_graph = self.to_focused_graph(&project, with_dependents);
        let (nodes, edges) = focused_graph.into_nodes_edges();

        let mut graph = DiGraph::with_capacity(nodes.len(), edges.len());
        let mut indexes = FxHashMap::default();
        let mut projects = FxHashMap::default();

        // The focused graph has different node inndexes,
        // so we need to update our internal structures to match
        for (i, node) in nodes.into_iter().enumerate() {
            let new_index = NodeIndex::from(i as u32);
            let old_index = node.weight;
            let id = &self.indexes[&old_index];

            indexes.insert(new_index, id.to_owned());

            projects.insert(
                id.to_owned(),
                ProjectNode {
                    index: new_index,
                    project: self.get_node_by_index(&old_index).to_owned(),
                },
            );

            graph.add_node(new_index);
        }

        for edge in edges {
            graph.update_edge(edge.source(), edge.target(), edge.weight);
        }

        let aliases = self
            .aliases
            .iter()
            .filter_map(|(alias, id)| {
                if projects.contains_key(id) {
                    Some((alias.to_owned(), id.to_owned()))
                } else {
                    None
                }
            })
            .collect();

        let mut focused = Self {
            aliases,
            context: self.context.clone(),
            indexes,
            default_id: self.default_id.clone(),
            fs_cache: Arc::clone(&self.fs_cache),
            nodes: projects,
            projects: Arc::clone(&self.projects),
            ..Default::default()
        };

        // The focused edges are a subset of already validated partitions,
        // so deriving them again cannot fail
        focused.set_graph(graph)?;

        Ok(focused)
    }

    fn label_index(&self, index: NodeIndex) -> String {
        self.indexes
            .get(&index)
            .map(|id| id.to_string())
            .unwrap_or_else(|| index.index().to_string())
    }

    fn internal_get(&self, id_or_alias: &str) -> miette::Result<Arc<Project>> {
        let id = self.resolve_id(id_or_alias);

        let project = match self.projects.entry_sync(id) {
            Entry::Occupied(entry) => Arc::clone(entry.get()),
            Entry::Vacant(entry) => {
                let expander = ProjectExpander::new(ProjectExpanderContext {
                    aliases: self.aliases(),
                    workspace_root: &self.context.workspace_root,
                });

                let project = Arc::new(expander.expand(self.get_unexpanded(entry.key())?)?);

                entry.insert_entry(Arc::clone(&project));

                project
            }
        };

        Ok(project)
    }

    fn internal_search(&self, search: &Path) -> miette::Result<Arc<String>> {
        let cache_key = search.to_path_buf();

        let cache = match self.fs_cache.entry_sync(cache_key) {
            Entry::Occupied(entry) => Arc::clone(entry.get()),
            Entry::Vacant(entry) => {
                // Find the deepest matching path in case sub-projects are being used
                let mut remaining_length = 1000; // Start with a really fake number
                let mut possible_id = String::new();

                for (id, node) in &self.nodes {
                    if !search.starts_with(node.project.source.as_str()) {
                        continue;
                    }

                    if let Ok(diff) = search.relative_to(node.project.source.as_str()) {
                        let diff_comps = diff.components().count();

                        // Exact match, abort
                        if diff_comps == 0 {
                            possible_id = id.as_str().to_owned();
                            break;
                        }

                        if diff_comps < remaining_length {
                            remaining_length = diff_comps;
                            possible_id = id.as_str().to_owned();
                        }
                    }
                }

                if possible_id.is_empty() {
                    return Err(ProjectGraphError::MissingFromPath {
                        dir: search.to_path_buf(),
                    }
                    .into());
                }

                let id = Arc::new(possible_id);

                entry.insert_entry(Arc::clone(&id));

                id
            }
        };

        Ok(cache)
    }

    pub fn resolve_id(&self, id_or_alias: &str) -> Id {
        Id::raw(if self.nodes.contains_key(id_or_alias) {
            id_or_alias
        } else if let Some(id) = self.aliases.get(id_or_alias) {
            id.as_str()
        } else {
            id_or_alias
        })
    }
}

impl GraphData<Project, DependencyScope, Id> for ProjectGraph {
    fn get_graph(&self) -> &DiGraph<NodeIndex, DependencyScope> {
        &self.graph
    }

    fn get_nodes(&self) -> FxHashMap<NodeIndex, &Project> {
        self.nodes
            .values()
            .map(|node| (node.index, &node.project))
            .collect()
    }

    fn get_node_by_index(&self, index: &NodeIndex) -> &Project {
        &self.nodes[&self.indexes[index]].project
    }

    fn get_node_key(&self, node: &Project) -> Id {
        node.id.clone()
    }
}

impl GraphConnections<Project, DependencyScope, Id> for ProjectGraph {
    fn get_node_index(&self, node: &Project) -> NodeIndex {
        self.nodes[&node.id].index
    }
}

impl GraphConversions<Project, DependencyScope, Id> for ProjectGraph {}

impl GraphToDot<Project, DependencyScope, Id> for ProjectGraph {}

impl GraphToJson<Project, DependencyScope, Id> for ProjectGraph {}

#[cfg(test)]
mod tests {
    use super::*;

    fn loop_graph(
        scope_ab: DependencyScope,
        scope_ba: DependencyScope,
    ) -> DiGraph<NodeIndex, DependencyScope> {
        let mut graph = DiGraph::new();
        let a = graph.add_node(NodeIndex::new(0));
        let b = graph.add_node(NodeIndex::new(1));

        graph.add_edge(a, b, scope_ab);
        graph.add_edge(b, a, scope_ba);
        graph
    }

    #[test]
    fn routes_scopes_into_expected_partitions() {
        assert!(is_production_scope(&DependencyScope::Production));
        assert!(is_production_scope(&DependencyScope::Peer));
        assert!(!is_production_scope(&DependencyScope::Build));
        assert!(!is_production_scope(&DependencyScope::Development));
        assert!(!is_production_scope(&DependencyScope::Root));

        assert_eq!(
            ScopePartition::of(&DependencyScope::Production),
            ScopePartition::Production
        );
        assert_eq!(
            ScopePartition::of(&DependencyScope::Peer),
            ScopePartition::Production
        );
        assert_eq!(
            ScopePartition::of(&DependencyScope::Build),
            ScopePartition::Development
        );
        assert_eq!(
            ScopePartition::of(&DependencyScope::Development),
            ScopePartition::Development
        );
        assert_eq!(
            ScopePartition::of(&DependencyScope::Root),
            ScopePartition::Development
        );
    }

    #[test]
    fn toposorts_partitions_in_dependency_first_order() {
        let mut project_graph = ProjectGraph::default();

        // a -> b (production), b -> a (development)
        project_graph
            .set_graph(loop_graph(
                DependencyScope::Production,
                DependencyScope::Development,
            ))
            .unwrap();
        project_graph
            .indexes
            .insert(NodeIndex::new(0), Id::raw("a"));
        project_graph
            .indexes
            .insert(NodeIndex::new(1), Id::raw("b"));

        // The union cycles, but each partition can still be sorted
        assert_eq!(
            project_graph.partitioned_toposort(ScopePartition::Production),
            [Id::raw("b"), Id::raw("a")]
        );
        assert_eq!(
            project_graph.partitioned_toposort(ScopePartition::Development),
            [Id::raw("a"), Id::raw("b")]
        );
    }

    #[test]
    fn would_cycle_for_self_loops_in_either_partition() {
        let mut graph = DiGraph::<(), DependencyScope>::new();
        let a = graph.add_node(());

        assert!(would_cycle_in_scope(
            &graph,
            a,
            a,
            &DependencyScope::Production
        ));
        assert!(would_cycle_in_scope(&graph, a, a, &DependencyScope::Build));
    }

    #[test]
    fn set_graph_allows_cross_partition_cycles() {
        let mut project_graph = ProjectGraph::default();

        project_graph
            .set_graph(loop_graph(
                DependencyScope::Production,
                DependencyScope::Development,
            ))
            .unwrap();

        assert_eq!(project_graph.production_graph().edge_count(), 1);
        assert_eq!(project_graph.development_graph().edge_count(), 1);
    }

    #[test]
    fn set_graph_errors_for_production_partition_cycles() {
        let mut project_graph = ProjectGraph::default();

        let error = project_graph
            .set_graph(loop_graph(
                DependencyScope::Production,
                DependencyScope::Peer,
            ))
            .unwrap_err();

        assert!(error.to_string().contains("would introduce a cycle"));
    }

    #[test]
    fn set_graph_errors_for_development_partition_cycles() {
        let mut project_graph = ProjectGraph::default();

        let error = project_graph
            .set_graph(loop_graph(
                DependencyScope::Build,
                DependencyScope::Development,
            ))
            .unwrap_err();

        assert!(error.to_string().contains("would introduce a cycle"));
    }
}
