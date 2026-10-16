//! Stages 3 and 4: assemble the selected instances into a graph, then add the
//! serial ordering edges, and freeze it.
//!
//! Every real relation is added before any preference: first the relations of
//! the task instances (in the order they were linked, so a cycle is reported
//! for the same relation as always), then the present dependencies of tasks
//! inserted without their dependencies, and last the serial ordering edges,
//! which are skipped when they would introduce a cycle, and never removed.

use super::select::{SerialLink, TaskPlan, TaskRelation};
use crate::action_graph::ActionGraph;
use crate::action_graph_error::ActionGraphError;
use daggy::{Dag, Walker};
use moon_action::ActionNode;
use moon_config::{TaskDependencyConfig, TaskDependencyType};
use moon_task::Target;
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use tracing::{trace, warn};

pub struct AssembledGraph {
    pub graph: ActionGraph,
    /// Dependencies of tasks inserted without them, that have no instance.
    pub ignored_dependencies: FxHashMap<Target, FxHashSet<Target>>,
    /// Targets depended on with `wait` by tasks inserted without dependencies.
    pub wait_targets: FxHashSet<Target>,
}

#[derive(Default)]
struct GraphAssembler {
    graph: Dag<NodeIndex, TaskDependencyType>,
    nodes: Vec<ActionNode>,
    cleanup_indices: FxHashSet<NodeIndex>,

    // Edges that aren't followed when walking a dependency's subtree, as
    // they don't exist yet (or never) when the serial order is applied
    present_edges: FxHashSet<EdgeIndex>,
    serial_edges: FxHashSet<EdgeIndex>,
}

pub fn assemble(plan: &mut TaskPlan) -> miette::Result<AssembledGraph> {
    let mut assembler = GraphAssembler::default();

    for node in std::mem::take(&mut plan.instances) {
        assembler.add_node(node);
    }

    assembler.link_relations(&plan.relations)?;

    let (ignored_dependencies, wait_targets) =
        assembler.link_present_dependencies(&plan.no_dependencies_tasks);

    assembler.link_serial_links(&plan.serial_links);

    Ok(AssembledGraph {
        graph: assembler.freeze(),
        ignored_dependencies,
        wait_targets,
    })
}

impl GraphAssembler {
    fn add_node(&mut self, node: ActionNode) -> NodeIndex {
        // Node weights are their own index, as the action graph has always had
        let index = NodeIndex::new(self.graph.node_count());

        self.graph.add_node(index);
        self.nodes.push(node);

        index
    }

    fn freeze(self) -> ActionGraph {
        let nodes = self
            .nodes
            .into_iter()
            .enumerate()
            .map(|(index, node)| (NodeIndex::new(index), node))
            .collect();

        ActionGraph::new_with_cleanups(self.graph, nodes, self.cleanup_indices)
    }

    fn label(&self, index: NodeIndex) -> String {
        self.nodes[index.index()].label()
    }

    /// Link every relation between task instances. A relation that would
    /// introduce a cycle is an error, as these are real dependencies.
    fn link_relations(&mut self, relations: &[TaskRelation]) -> miette::Result<()> {
        for relation in relations {
            let from = NodeIndex::new(relation.from);
            let to = NodeIndex::new(relation.to);

            // Cleanup actions run *after* the node they clean up, so the edge
            // points from the cleanup action to the node. The cleanup action may
            // also depend on the node in some other way, which is a separate edge
            if !self.has_edge_of_type(from, to, relation.weight) {
                self.link_edge(from, to, relation.weight)?;
            }

            if matches!(relation.weight, TaskDependencyType::Cleanup) {
                self.cleanup_indices.insert(from);
            }
        }

        Ok(())
    }

    fn link_edge(
        &mut self,
        from: NodeIndex,
        to: NodeIndex,
        weight: TaskDependencyType,
    ) -> miette::Result<()> {
        if self.graph.add_edge(from, to, weight).is_err() {
            return Err(ActionGraphError::WouldCycle {
                source_action: self.label(from),
                target_action: self.label(to),
                cycle: self.find_cycle(from, to),
            }
            .into());
        }

        trace!(
            index = from.index(),
            requires = ?[to.index()],
            "Linking requirements for index"
        );

        Ok(())
    }

    /// Describe the cycle that an edge from `from` to `to` would introduce,
    /// as `to` already reaches `from`.
    fn find_cycle(&self, from: NodeIndex, to: NodeIndex) -> Option<String> {
        let graph = self.graph.graph();
        let mut parents = FxHashMap::<NodeIndex, NodeIndex>::default();
        let mut queue = VecDeque::from([to]);

        parents.insert(to, to);

        while let Some(index) = queue.pop_front() {
            if index == from {
                let mut path = vec![from];
                let mut current = from;

                while current != to {
                    current = parents[&current];
                    path.push(current);
                }

                path.reverse();
                path.insert(0, from);

                return Some(format!(
                    "The cycle is {}",
                    path.into_iter()
                        .map(|index| self.label(index))
                        .collect::<Vec<_>>()
                        .join(" → ")
                ));
            }

            for edge in graph.edges_directed(index, Direction::Outgoing) {
                if let Entry::Vacant(entry) = parents.entry(edge.target()) {
                    entry.insert(index);
                    queue.push_back(edge.target());
                }
            }
        }

        None
    }

    /// Whether an edge of the provided type already links the two nodes. Edges
    /// of different types may link the same nodes (a task that both requires
    /// and cleans up after another), and the pipeline honors all of them.
    fn has_edge_of_type(&self, from: NodeIndex, to: NodeIndex, weight: TaskDependencyType) -> bool {
        self.graph
            .graph()
            .edges_connecting(from, to)
            .any(|edge| *edge.weight() == weight)
    }

    /// Link the dependencies of tasks that were inserted without them
    /// (`no_dependencies`) to the instances that exist, as the ordering between
    /// tasks that do run must still be honored. When a dependency has multiple
    /// instances (different args or env), the ones that match the dependency's
    /// config are preferred, otherwise all of them are linked. Dependencies
    /// without an instance are ignored, so that the task doesn't wait on (or
    /// hash) them. Serial ordering is not applied to these edges.
    fn link_present_dependencies(
        &mut self,
        no_dependencies_tasks: &FxHashMap<Target, Vec<TaskDependencyConfig>>,
    ) -> (FxHashMap<Target, FxHashSet<Target>>, FxHashSet<Target>) {
        let mut ignored_dependencies = FxHashMap::<Target, FxHashSet<Target>>::default();
        let mut wait_targets = FxHashSet::default();

        if no_dependencies_tasks.is_empty() {
            return (ignored_dependencies, wait_targets);
        }

        let mut target_indices = FxHashMap::<Target, Vec<NodeIndex>>::default();

        for (index, node) in self.nodes.iter().enumerate() {
            if let ActionNode::RunTask(inner) = node {
                target_indices
                    .entry(inner.target.clone())
                    .or_default()
                    .push(NodeIndex::new(index));
            }
        }

        let mut tasks = no_dependencies_tasks.iter().collect::<Vec<_>>();

        // Link in a deterministic order, so that the graph's edge order is stable
        tasks.sort_by(|a, b| a.0.cmp(b.0));

        for (target, deps) in tasks {
            let Some(owner_indices) = target_indices.get(target) else {
                continue;
            };

            for dep in deps {
                let Some(dep_indices) = target_indices.get(&dep.target) else {
                    ignored_dependencies
                        .entry(target.clone())
                        .or_default()
                        .insert(dep.target.clone());

                    continue;
                };

                // The optional type is only a marker within the task
                // graph, the action graph has always used required
                let weight = if dep.type_of.is_required_type() {
                    TaskDependencyType::Required
                } else {
                    dep.type_of
                };

                let matching_indices = dep_indices
                    .iter()
                    .copied()
                    .filter(|index| {
                        matches!(
                            &self.nodes[index.index()],
                            ActionNode::RunTask(inner)
                                if inner.args == dep.args && inner.env == dep.env
                        )
                    })
                    .collect::<Vec<_>>();

                let dep_indices = if matching_indices.is_empty() {
                    dep_indices.clone()
                } else {
                    matching_indices
                };

                if matches!(weight, TaskDependencyType::Wait) {
                    wait_targets.insert(dep.target.clone());
                }

                for owner_index in owner_indices {
                    if dep_indices
                        .iter()
                        .any(|dep_index| self.has_edge_of_type(*owner_index, *dep_index, weight))
                    {
                        continue;
                    }

                    for dep_index in &dep_indices {
                        if owner_index == dep_index {
                            continue;
                        }

                        match self.graph.add_edge(*owner_index, *dep_index, weight) {
                            Ok(edge_index) => {
                                self.present_edges.insert(edge_index);
                            }
                            // The task graph is acyclic, so this is unexpected
                            Err(_) => {
                                warn!(
                                    task_target = target.as_str(),
                                    dependency_target = dep.target.as_str(),
                                    "Unable to link dependency: adding a relationship from action {} to {} would introduce a cycle",
                                    self.label(*owner_index),
                                    self.label(*dep_index),
                                );
                            }
                        };
                    }
                }
            }
        }

        (ignored_dependencies, wait_targets)
    }

    /// Apply the serial ordering links, in the order that they were made.
    fn link_serial_links(&mut self, links: &[SerialLink]) {
        for link in links {
            self.link_serial_required_edges(
                NodeIndex::new(link.index),
                link.previous.map(NodeIndex::new),
                link.previous_standard.map(NodeIndex::new),
            );
        }
    }

    /// Add serial ordering edges from every task within `index`'s dependency
    /// subtree (including `index` itself) to the previous serial dependency, so
    /// that the whole subtree runs after it. Only `RunTask` nodes are ordered.
    ///
    /// Non-persistent tasks are ordered against `previous_standard` — the
    /// previous dependency that actually completes — instead of `previous`,
    /// as a persistent dependency never completes and so cannot order them.
    ///
    /// The walk only follows real dependency edges that block a node from
    /// running: never serial edges (following one would escape `index`'s real
    /// subtree), present dependency edges (which never took part in the serial
    /// order), `cleanup` edges (which point back to the task that owns the
    /// cleanup), or `wait` edges (which don't run to completion first).
    fn link_serial_required_edges(
        &mut self,
        index: NodeIndex,
        previous: Option<NodeIndex>,
        previous_standard: Option<NodeIndex>,
    ) {
        if previous.is_none() && previous_standard.is_none() {
            return;
        }

        let mut visited = FxHashSet::default();
        let mut ordered = vec![];
        let mut stack = vec![index];

        // Collect the subtree first, then link, as linking adds edges that
        // must not pollute the walk. `ordered` keeps the link order stable
        while let Some(node_index) = stack.pop() {
            if !visited.insert(node_index) {
                continue;
            }

            ordered.push(node_index);

            let mut children = self.graph.children(node_index);

            while let Some((edge_index, child_index)) = children.walk_next(&self.graph) {
                if !self.serial_edges.contains(&edge_index)
                    && !self.present_edges.contains(&edge_index)
                    && self
                        .graph
                        .edge_weight(edge_index)
                        .is_some_and(|weight| weight.is_required_type())
                    && matches!(self.nodes[child_index.index()], ActionNode::RunTask(_))
                {
                    stack.push(child_index);
                }
            }
        }

        for node_index in ordered {
            // A persistent action is marked as completed the moment it is
            // dispatched, so a non-persistent action ordered after one would
            // simply run immediately. Order those against the previous
            // dependency that does complete
            let anchor = if self.nodes[node_index.index()].is_persistent() {
                previous
            } else {
                previous_standard
            };

            if let Some(anchor) = anchor
                && anchor != node_index
            {
                self.try_link_serial_edge(node_index, anchor);
            }
        }
    }

    /// Try to add a serial ordering edge. It's a preference, so it's skipped
    /// when a required edge already links the nodes, or when it would introduce
    /// a cycle, which happens when the same node is in the serial chains of
    /// several parents, or when the nodes depend on each other the other way.
    fn try_link_serial_edge(&mut self, index: NodeIndex, edge: NodeIndex) {
        if self.has_edge_of_type(index, edge, TaskDependencyType::Required) {
            return;
        }

        if let Ok(edge_index) = self
            .graph
            .add_edge(index, edge, TaskDependencyType::Required)
        {
            self.serial_edges.insert(edge_index);

            trace!(
                index = index.index(),
                requires = ?[edge.index()],
                "Linking serial requirements for index"
            );
        }
    }
}
