use crate::errors::WorkspaceError;
use moon_logger::{color, debug, trace};
use moon_project::{
    ProjectGraph, ProjectID, Target, TargetError, TargetID, TargetProject, TouchedFilePaths,
};
use petgraph::algo::toposort;
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use petgraph::Graph;
use std::collections::{HashMap, HashSet};

pub use petgraph::graph::NodeIndex;

const TARGET: &str = "moon:dep-graph";

pub enum Node {
    InstallNodeDeps,
    RunTarget(TargetID),
    SetupToolchain,
    SyncProject(ProjectID),
}

impl Node {
    pub fn label(&self) -> String {
        match self {
            Node::InstallNodeDeps => String::from("InstallNodeDeps"),
            Node::RunTarget(id) => format!("RunTarget({})", id),
            Node::SetupToolchain => String::from("SetupToolchain"),
            Node::SyncProject(id) => format!("SyncProject({})", id),
        }
    }
}

type GraphType = DiGraph<Node, ()>;
type BatchedTopoSort = Vec<Vec<NodeIndex>>;

/// A directed acyclic graph (DAG) for the work that needs to be processed, based on a
/// project or task's dependency chain. This is also known as a "task graph" (not to
/// be confused with ours) or a "dependency graph".
pub struct DepGraph {
    pub graph: GraphType,

    /// Mapping of IDs to existing node indices.
    index_cache: HashMap<String, NodeIndex>,

    /// Reference node for the "install node deps" task.
    install_node_deps_index: NodeIndex,

    /// Reference node for the "setup toolchain" task.
    setup_toolchain_index: NodeIndex,
}

impl DepGraph {
    pub fn default() -> Self {
        debug!(target: TARGET, "Creating dependency graph",);

        let mut graph: GraphType = Graph::new();

        // Toolchain must be setup first
        let setup_toolchain_index = graph.add_node(Node::SetupToolchain);

        // Deps can be installed *after* the toolchain exists
        let install_node_deps_index = graph.add_node(Node::InstallNodeDeps);

        graph.add_edge(install_node_deps_index, setup_toolchain_index, ());

        DepGraph {
            graph,
            index_cache: HashMap::new(),
            install_node_deps_index,
            setup_toolchain_index,
        }
    }

    pub fn get_node_from_index(&self, index: NodeIndex) -> Option<&Node> {
        self.graph.node_weight(index)
    }

    pub fn sort_topological(&self) -> Result<Vec<NodeIndex>, WorkspaceError> {
        let list = match toposort(&self.graph, None) {
            Ok(nodes) => nodes,
            Err(error) => {
                return Err(WorkspaceError::DepGraphCycleDetected(
                    self.get_node_from_index(error.node_id()).unwrap().label(),
                ));
            }
        };

        Ok(list.into_iter().rev().collect())
    }

    pub fn sort_batched_topological(&self) -> Result<BatchedTopoSort, WorkspaceError> {
        let mut batches: BatchedTopoSort = vec![];

        // Count how many times an index is referened across nodes and edges
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

    pub fn run_target(
        &mut self,
        target: &Target,
        projects: &ProjectGraph,
        touched_files: Option<&TouchedFilePaths>,
    ) -> Result<usize, WorkspaceError> {
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
    ) -> Result<(), WorkspaceError> {
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

    pub fn sync_project(
        &mut self,
        project_id: &str,
        projects: &ProjectGraph,
    ) -> Result<NodeIndex, WorkspaceError> {
        if self.index_cache.contains_key(project_id) {
            return Ok(*self.index_cache.get(project_id).unwrap());
        }

        trace!(
            target: TARGET,
            "Syncing project {} configs and dependencies",
            color::id(project_id),
        );

        // Force load project into the graph
        let project = projects.load(project_id)?;

        // Sync can be run in parallel while deps are installing
        let node_index = self
            .graph
            .add_node(Node::SyncProject(project_id.to_owned()));

        self.graph
            .add_edge(node_index, self.setup_toolchain_index, ());

        // Cache so we don't sync the same project multiple times
        self.index_cache.insert(project_id.to_owned(), node_index);

        // But we need to wait on all dependent nodes
        for dep_id in projects.get_dependencies_of(&project)? {
            let dep_node_index = self.sync_project(&dep_id, projects)?;
            self.graph.add_edge(node_index, dep_node_index, ());
        }

        Ok(node_index)
    }

    pub fn to_dot(&self) -> String {
        let graph = self.graph.map(|_, n| n.label(), |_, e| e);
        let dot = Dot::with_config(&graph, &[Config::EdgeNoLabel]);

        format!("{:?}", dot)
    }

    fn detect_cycle(&self) -> Result<(), WorkspaceError> {
        use petgraph::algo::kosaraju_scc;

        let scc = kosaraju_scc(&self.graph);
        let cycle = scc
            .last()
            .unwrap()
            .iter()
            .map(|i| self.get_node_from_index(*i).unwrap().label())
            .collect::<Vec<String>>()
            .join(" → ");

        Err(WorkspaceError::DepGraphCycleDetected(cycle))
    }

    fn insert_target(
        &mut self,
        project_id: &str,
        task_id: &str,
        projects: &ProjectGraph,
        touched_files: Option<&TouchedFilePaths>,
    ) -> Result<Option<NodeIndex>, WorkspaceError> {
        let target_id = Target::format(project_id, task_id)?;

        if self.index_cache.contains_key(&target_id) {
            return Ok(Some(*self.index_cache.get(&target_id).unwrap()));
        }

        let project = projects.load(project_id)?;

        // Compare against touched files if provided
        if let Some(touched) = touched_files {
            let globally_affected = projects.is_globally_affected(touched);

            if globally_affected {
                trace!(
                    target: TARGET,
                    "Moon files touched, marking all targets as affected",
                );
            }

            // Validate project first
            if !globally_affected && !project.is_affected(touched) {
                trace!(
                    target: TARGET,
                    "Project {} not affected based on touched files, skipping",
                    color::id(project_id),
                );

                return Ok(None);
            }

            // Validate task exists for project
            if !globally_affected && !project.get_task(task_id)?.is_affected(touched)? {
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

        // We should sync projects *before* running targets
        let project_node = self.sync_project(&project.id, projects)?;
        let node = self.graph.add_node(Node::RunTarget(target_id.to_owned()));

        self.graph.add_edge(node, self.install_node_deps_index, ());
        self.graph.add_edge(node, project_node, ());

        // Also cache so we don't run the same target multiple times
        self.index_cache.insert(target_id.to_owned(), node);

        // And we also need to wait on all dependent nodes
        let task = project.get_task(task_id)?;

        if !task.deps.is_empty() {
            let dep_names: Vec<String> = task
                .deps
                .clone()
                .into_iter()
                .map(|d| color::symbol(&d))
                .collect();

            trace!(
                target: TARGET,
                "Adding dependencies {} from target {}",
                dep_names.join(", "),
                color::target(&target_id),
            );

            for dep_target_id in &task.deps {
                let dep_target = Target::parse(dep_target_id)?;

                if let Some(dep_node) = self.insert_target(
                    &dep_target.project_id.unwrap(),
                    &dep_target.task_id,
                    projects,
                    touched_files,
                )? {
                    self.graph.add_edge(node, dep_node, ());
                }
            }
        }

        Ok(Some(node))
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

    fn create_tasks_project_graph() -> ProjectGraph {
        let global_config = GlobalProjectConfig {
            file_groups: HashMap::from([("sources".to_owned(), vec!["src/**/*".to_owned()])]),
            ..GlobalProjectConfig::default()
        };

        ProjectGraph::new(
            &get_fixtures_dir("tasks"),
            global_config,
            &HashMap::from([
                ("basic".to_owned(), "basic".to_owned()),
                ("build-a".to_owned(), "build-a".to_owned()),
                ("build-b".to_owned(), "build-b".to_owned()),
                ("build-c".to_owned(), "build-c".to_owned()),
                ("chain".to_owned(), "chain".to_owned()),
                ("cycle".to_owned(), "cycle".to_owned()),
                ("inputA".to_owned(), "input-a".to_owned()),
                ("inputB".to_owned(), "input-b".to_owned()),
                ("inputC".to_owned(), "input-c".to_owned()),
                ("mergeAppend".to_owned(), "merge-append".to_owned()),
                ("mergePrepend".to_owned(), "merge-prepend".to_owned()),
                ("mergeReplace".to_owned(), "merge-replace".to_owned()),
                ("no-tasks".to_owned(), "no-tasks".to_owned()),
            ]),
        )
    }

    fn sort_batches(batches: BatchedTopoSort) -> BatchedTopoSort {
        let mut list: BatchedTopoSort = vec![];

        for batch in batches {
            let mut new_batch = batch.clone();
            new_batch.sort();
            list.push(new_batch);
        }

        list
    }

    #[test]
    fn default_graph() {
        let graph = DepGraph::default();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![NodeIndex::new(0), NodeIndex::new(1)]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![vec![NodeIndex::new(0)], vec![NodeIndex::new(1)]]
        );
    }

    #[test]
    #[should_panic(
        expected = "CycleDetected(\"RunTarget(cycle:a) -> RunTarget(cycle:b) -> RunTarget(cycle:c)\")"
    )]
    fn detects_cycles() {
        let projects = create_tasks_project_graph();

        let mut graph = DepGraph::default();
        graph
            .run_target(&Target::new("cycle", "a").unwrap(), &projects, None)
            .unwrap();
        graph
            .run_target(&Target::new("cycle", "b").unwrap(), &projects, None)
            .unwrap();
        graph
            .run_target(&Target::new("cycle", "c").unwrap(), &projects, None)
            .unwrap();

        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![vec![NodeIndex::new(0)], vec![NodeIndex::new(1)]]
        );
    }

    mod run_target {
        use super::*;

        #[test]
        fn single_targets() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph
                .run_target(&Target::new("tasks", "test").unwrap(), &projects, None)
                .unwrap();
            graph
                .run_target(&Target::new("tasks", "lint").unwrap(), &projects, None)
                .unwrap();

            assert_snapshot!(graph.to_dot());

            assert_eq!(
                graph.sort_topological().unwrap(),
                vec![
                    NodeIndex::new(0),
                    NodeIndex::new(1),
                    NodeIndex::new(2), // sync project
                    NodeIndex::new(3), // test
                    NodeIndex::new(4), // lint
                ]
            );
            assert_eq!(
                sort_batches(graph.sort_batched_topological().unwrap()),
                vec![
                    vec![NodeIndex::new(0)],
                    vec![NodeIndex::new(1), NodeIndex::new(2)],
                    vec![NodeIndex::new(3), NodeIndex::new(4)]
                ]
            );
        }

        #[test]
        fn deps_chain_target() {
            let projects = create_tasks_project_graph();

            let mut graph = DepGraph::default();
            graph
                .run_target(&Target::new("basic", "test").unwrap(), &projects, None)
                .unwrap();
            graph
                .run_target(&Target::new("basic", "lint").unwrap(), &projects, None)
                .unwrap();
            graph
                .run_target(&Target::new("chain", "a").unwrap(), &projects, None)
                .unwrap();

            assert_snapshot!(graph.to_dot());

            assert_eq!(
                graph.sort_topological().unwrap(),
                vec![
                    NodeIndex::new(0),
                    NodeIndex::new(1),
                    NodeIndex::new(2),  // sync project
                    NodeIndex::new(3),  // test
                    NodeIndex::new(4),  // lint
                    NodeIndex::new(5),  // sync project
                    NodeIndex::new(11), // f
                    NodeIndex::new(10), // e
                    NodeIndex::new(9),  // d
                    NodeIndex::new(8),  // c
                    NodeIndex::new(7),  // b
                    NodeIndex::new(6),  // a
                ]
            );
            assert_eq!(
                sort_batches(graph.sort_batched_topological().unwrap()),
                vec![
                    vec![NodeIndex::new(0)],
                    vec![NodeIndex::new(1), NodeIndex::new(5)],
                    vec![NodeIndex::new(11)],
                    vec![NodeIndex::new(10)],
                    vec![NodeIndex::new(9)],
                    vec![NodeIndex::new(8)],
                    vec![NodeIndex::new(2), NodeIndex::new(7)],
                    vec![NodeIndex::new(3), NodeIndex::new(4), NodeIndex::new(6)]
                ]
            );
        }

        #[test]
        fn avoids_dupe_targets() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph
                .run_target(&Target::new("tasks", "lint").unwrap(), &projects, None)
                .unwrap();
            graph
                .run_target(&Target::new("tasks", "lint").unwrap(), &projects, None)
                .unwrap();
            graph
                .run_target(&Target::new("tasks", "lint").unwrap(), &projects, None)
                .unwrap();

            assert_snapshot!(graph.to_dot());

            assert_eq!(
                graph.sort_topological().unwrap(),
                vec![
                    NodeIndex::new(0),
                    NodeIndex::new(1),
                    NodeIndex::new(2), // sync project
                    NodeIndex::new(3), // lint
                ]
            );
            assert_eq!(
                sort_batches(graph.sort_batched_topological().unwrap()),
                vec![
                    vec![NodeIndex::new(0)],
                    vec![NodeIndex::new(1), NodeIndex::new(2)],
                    vec![NodeIndex::new(3)]
                ]
            );
        }

        #[test]
        fn runs_all_projects_for_target_all_scope() {
            let projects = create_tasks_project_graph();

            let mut graph = DepGraph::default();
            graph
                .run_target(&Target::parse(":build").unwrap(), &projects, None)
                .unwrap();

            assert_snapshot!(graph.to_dot());

            assert_eq!(
                graph.sort_topological().unwrap(),
                vec![
                    NodeIndex::new(0),
                    NodeIndex::new(1),
                    NodeIndex::new(2), // sync project: basic
                    NodeIndex::new(3), // basic:build
                    NodeIndex::new(5), // sync project: build-c
                    NodeIndex::new(4), // sync project: build-a
                    NodeIndex::new(7), // build-c:build
                    NodeIndex::new(6), // build-a:build
                    NodeIndex::new(8), // sync project: build-b
                    NodeIndex::new(9), // build-b:build
                ]
            );
            assert_eq!(
                sort_batches(graph.sort_batched_topological().unwrap()),
                vec![
                    vec![NodeIndex::new(0)],
                    vec![NodeIndex::new(1), NodeIndex::new(2), NodeIndex::new(5)],
                    vec![
                        NodeIndex::new(3),
                        NodeIndex::new(4),
                        NodeIndex::new(7),
                        NodeIndex::new(8)
                    ],
                    vec![NodeIndex::new(6), NodeIndex::new(9)],
                ]
            );
        }

        #[test]
        #[should_panic(expected = "Project(Target(NoProjectDepsInRunContext))")]
        fn errors_for_target_deps_scope() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph
                .run_target(&Target::parse("^:lint").unwrap(), &projects, None)
                .unwrap();
        }

        #[test]
        #[should_panic(expected = "Project(Target(NoProjectSelfInRunContext))")]
        fn errors_for_target_self_scope() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph
                .run_target(&Target::parse("~:lint").unwrap(), &projects, None)
                .unwrap();
        }

        #[test]
        #[should_panic(expected = "Project(UnconfiguredID(\"unknown\"))")]
        fn errors_for_unknown_project() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph
                .run_target(&Target::new("unknown", "test").unwrap(), &projects, None)
                .unwrap();

            assert_snapshot!(graph.to_dot());
        }

        #[test]
        #[should_panic(expected = "Project(UnconfiguredTask(\"build\", \"tasks\"))")]
        fn errors_for_unknown_task() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph
                .run_target(&Target::new("tasks", "build").unwrap(), &projects, None)
                .unwrap();

            assert_snapshot!(graph.to_dot());
        }
    }

    mod run_target_if_touched {
        use super::*;

        #[test]
        fn skips_if_untouched_project() {
            let projects = create_tasks_project_graph();

            let mut touched_files = HashSet::new();
            touched_files.insert(get_fixtures_dir("tasks").join("input-a/a.ts"));
            touched_files.insert(get_fixtures_dir("tasks").join("input-c/c.ts"));

            let mut graph = DepGraph::default();
            graph
                .run_target(
                    &Target::new("inputA", "a").unwrap(),
                    &projects,
                    Some(&touched_files),
                )
                .unwrap();
            graph
                .run_target(
                    &Target::new("inputB", "b").unwrap(),
                    &projects,
                    Some(&touched_files),
                )
                .unwrap();

            assert_snapshot!(graph.to_dot());
        }

        #[test]
        fn skips_if_untouched_task() {
            let projects = create_tasks_project_graph();

            let mut touched_files = HashSet::new();
            touched_files.insert(get_fixtures_dir("tasks").join("input-a/a2.ts"));
            touched_files.insert(get_fixtures_dir("tasks").join("input-b/b2.ts"));
            touched_files.insert(get_fixtures_dir("tasks").join("input-c/any.ts"));

            let mut graph = DepGraph::default();
            graph
                .run_target(
                    &Target::new("inputA", "a").unwrap(),
                    &projects,
                    Some(&touched_files),
                )
                .unwrap();
            graph
                .run_target(
                    &Target::new("inputB", "b2").unwrap(),
                    &projects,
                    Some(&touched_files),
                )
                .unwrap();
            graph
                .run_target(
                    &Target::new("inputC", "c").unwrap(),
                    &projects,
                    Some(&touched_files),
                )
                .unwrap();

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

            assert_eq!(
                graph.sort_topological().unwrap(),
                vec![
                    NodeIndex::new(0),
                    NodeIndex::new(1),
                    NodeIndex::new(2),
                    NodeIndex::new(4), // noConfig
                    NodeIndex::new(3), // basic
                    NodeIndex::new(5), // emptyConfig
                ]
            );
            assert_eq!(
                sort_batches(graph.sort_batched_topological().unwrap()),
                vec![
                    vec![NodeIndex::new(0)],
                    vec![NodeIndex::new(4)],
                    vec![
                        NodeIndex::new(1),
                        NodeIndex::new(2),
                        NodeIndex::new(3),
                        NodeIndex::new(5)
                    ]
                ]
            );
        }

        #[test]
        fn projects_with_deps() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph.sync_project("foo", &projects).unwrap();
            graph.sync_project("bar", &projects).unwrap();
            graph.sync_project("baz", &projects).unwrap();
            graph.sync_project("basic", &projects).unwrap();

            // Not deterministic!
            // assert_snapshot!(graph.to_dot());

            assert_eq!(
                graph.sort_topological().unwrap(),
                vec![
                    NodeIndex::new(0),
                    NodeIndex::new(1),
                    NodeIndex::new(3), // bar
                    NodeIndex::new(4), // baz
                    NodeIndex::new(2), // foo
                    NodeIndex::new(6), // emptyConfig
                    NodeIndex::new(5), // basic
                ]
            );
            assert_eq!(
                sort_batches(graph.sort_batched_topological().unwrap()),
                vec![
                    vec![NodeIndex::new(0)],
                    vec![NodeIndex::new(3), NodeIndex::new(4), NodeIndex::new(6)],
                    vec![NodeIndex::new(1), NodeIndex::new(2), NodeIndex::new(5)]
                ]
            );
        }

        #[test]
        fn projects_with_tasks() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph.sync_project("noConfig", &projects).unwrap();
            graph.sync_project("tasks", &projects).unwrap();

            assert_snapshot!(graph.to_dot());

            assert_eq!(
                graph.sort_topological().unwrap(),
                vec![
                    NodeIndex::new(0),
                    NodeIndex::new(1),
                    NodeIndex::new(2),
                    NodeIndex::new(3),
                ]
            );
            assert_eq!(
                sort_batches(graph.sort_batched_topological().unwrap()),
                vec![
                    vec![NodeIndex::new(0)],
                    vec![NodeIndex::new(1), NodeIndex::new(2), NodeIndex::new(3)]
                ]
            );
        }

        #[test]
        fn avoids_dupe_projects() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph.sync_project("advanced", &projects).unwrap();
            graph.sync_project("advanced", &projects).unwrap();
            graph.sync_project("advanced", &projects).unwrap();

            assert_snapshot!(graph.to_dot());
        }

        #[test]
        #[should_panic(expected = "Project(UnconfiguredID(\"unknown\"))")]
        fn errors_for_unknown_project() {
            let projects = create_project_graph();

            let mut graph = DepGraph::default();
            graph.sync_project("unknown", &projects).unwrap();

            assert_snapshot!(graph.to_dot());
        }
    }
}
