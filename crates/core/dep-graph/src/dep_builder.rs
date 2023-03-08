use crate::dep_graph::{DepGraph, DepGraphType, IndicesType};
use crate::errors::DepGraphError;
use moon_action::ActionNode;
use moon_logger::{color, debug, map_list, trace};
use moon_platform::{PlatformManager, Runtime};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_target::{Target, TargetError, TargetProjectScope};
use moon_task::{Task, TouchedFilePaths};
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use rustc_hash::{FxHashMap, FxHashSet};
use std::mem;
use std::path::Path;

const LOG_TARGET: &str = "moon:dep-graph";

type RuntimePair = (Runtime, Runtime);

/// A directed acyclic graph (DAG) for the work that needs to be processed, based on a
/// project or task's dependency chain. This is also known as a "task graph" (not to
/// be confused with our tasks) or a "dependency graph".
pub struct DepGraphBuilder<'ws> {
    graph: DepGraphType,
    indices: IndicesType,
    platforms: &'ws PlatformManager,
    project_graph: &'ws ProjectGraph,
    runtimes: FxHashMap<String, RuntimePair>,
    workspace_root: &'ws Path,
}

impl<'ws> DepGraphBuilder<'ws> {
    pub fn new(
        workspace_root: &'ws Path,
        platforms: &'ws PlatformManager,
        project_graph: &'ws ProjectGraph,
    ) -> Self {
        debug!(target: LOG_TARGET, "Creating dependency graph",);

        DepGraphBuilder {
            graph: Graph::new(),
            indices: FxHashMap::default(),
            platforms,
            project_graph,
            runtimes: FxHashMap::default(),
            workspace_root,
        }
    }

    pub fn build(&mut self) -> DepGraph {
        DepGraph::new(mem::take(&mut self.graph), mem::take(&mut self.indices))
    }

    pub fn get_index_from_node(&self, node: &ActionNode) -> Option<&NodeIndex> {
        self.indices.get(node)
    }

    // Projects support overriding the the version of their language (tool),
    // so we need to account for this via the runtime. However, some actions require
    // the workspace version of the language, so we must extract 2 runtimes here.
    pub fn get_runtimes_from_project(
        &mut self,
        project: &Project,
        task: Option<&Task>,
    ) -> (Runtime, Runtime) {
        let key = match task {
            Some(task) => task.target.id.clone(),
            None => project.id.clone(),
        };

        if let Some(pair) = self.runtimes.get(&key) {
            return pair.clone();
        }

        let mut project_runtime = Runtime::System;
        let mut workspace_runtime = Runtime::System;

        if let Some(platform) = self.platforms.find(|p| match task {
            Some(task) => p.matches(&task.platform, None),
            None => p.matches(&project.language.clone().into(), None),
        }) {
            project_runtime = platform.get_runtime_from_config(Some(&project.config));
            workspace_runtime = platform.get_runtime_from_config(None);
        }

        let pair = (project_runtime, workspace_runtime);

        self.runtimes.insert(key, pair.clone());

        pair
    }

    pub fn install_deps(
        &mut self,
        project: &Project,
        task: Option<&Task>,
    ) -> Result<NodeIndex, DepGraphError> {
        let (project_runtime, workspace_runtime) = self.get_runtimes_from_project(project, task);
        let mut installs_in_project = false;

        // If project is NOT in the package manager workspace, then we should
        // install dependencies in the project, not the workspace root.
        if let Ok(platform) = self.platforms.get(project.language.clone()) {
            if !platform.is_project_in_dependency_workspace(project)? {
                installs_in_project = true;
            }
        }

        // When installing dependencies in the project, we will use the
        // overridden version if it is available. Otherwise when installing
        // in the root, we should *always* use the workspace version.
        Ok(if installs_in_project {
            self.install_project_deps(&project_runtime, &project.id)
        } else {
            self.install_workspace_deps(&workspace_runtime)
        })
    }

    pub fn install_project_deps(&mut self, runtime: &Runtime, project_id: &str) -> NodeIndex {
        let node = ActionNode::InstallProjectDeps(runtime.clone(), project_id.to_owned());

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        trace!(
            target: LOG_TARGET,
            "Adding {} to graph",
            color::muted(node.label())
        );

        // Before we install deps, we must ensure the language has been installed
        let setup_tool_index = self.setup_tool(runtime);
        let index = self.insert_node(&node);

        self.graph.add_edge(index, setup_tool_index, ());

        index
    }

    pub fn install_workspace_deps(&mut self, runtime: &Runtime) -> NodeIndex {
        let node = ActionNode::InstallDeps(runtime.clone());

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        trace!(
            target: LOG_TARGET,
            "Adding {} to graph",
            color::muted(node.label())
        );

        // Before we install deps, we must ensure the language has been installed
        let setup_tool_index = self.setup_tool(runtime);
        let index = self.insert_node(&node);

        self.graph.add_edge(index, setup_tool_index, ());

        index
    }

    pub fn run_dependents_for_target<T: AsRef<Target>>(
        &mut self,
        target: T,
    ) -> Result<(), DepGraphError> {
        let target = target.as_ref();

        trace!(
            target: LOG_TARGET,
            "Adding dependents to run for target {}",
            color::target(&target.id),
        );

        let (project_id, task_id) = target.ids()?;
        let project = self.project_graph.get(&project_id)?;
        let dependents = self.project_graph.get_dependents_of(project)?;

        for dependent_id in dependents {
            let dep_project = self.project_graph.get(&dependent_id)?;

            if let Some(dep_task) = dep_project.tasks.get(&task_id) {
                self.run_target(&dep_task.target, None)?;
            }
        }

        Ok(())
    }

    pub fn run_target<T: AsRef<Target>>(
        &mut self,
        target: T,
        touched_files: Option<&TouchedFilePaths>,
    ) -> Result<(FxHashSet<Target>, FxHashSet<NodeIndex>), DepGraphError> {
        let target = target.as_ref();
        let mut inserted_targets = FxHashSet::default();
        let mut inserted_indexes = FxHashSet::default();

        match &target.project {
            // :task
            TargetProjectScope::All => {
                for project in self.project_graph.get_all()? {
                    if project.tasks.contains_key(&target.task_id) {
                        let all_target = Target::new(&project.id, &target.task_id)?;

                        if let Some(index) =
                            self.run_target_by_project(&all_target, project, touched_files)?
                        {
                            inserted_targets.insert(all_target);
                            inserted_indexes.insert(index);
                        }
                    }
                }
            }
            // ^:task
            TargetProjectScope::Deps => {
                target.fail_with(TargetError::NoProjectDepsInRunContext)?;
            }
            // project:task
            TargetProjectScope::Id(project_id) => {
                let project = self.project_graph.get(project_id)?;
                let task = project.get_task(&target.task_id)?;

                if let Some(index) =
                    self.run_target_by_project(&task.target, project, touched_files)?
                {
                    inserted_targets.insert(task.target.to_owned());
                    inserted_indexes.insert(index);
                }
            }
            // ~:task
            TargetProjectScope::OwnSelf => {
                target.fail_with(TargetError::NoProjectSelfInRunContext)?;
            }
        };

        Ok((inserted_targets, inserted_indexes))
    }

    pub fn run_target_by_project<T: AsRef<Target>>(
        &mut self,
        target: T,
        project: &Project,
        touched_files: Option<&TouchedFilePaths>,
    ) -> Result<Option<NodeIndex>, DepGraphError> {
        let target = target.as_ref();
        let task = project.get_task(&target.task_id)?;
        let (runtime, _) = self.get_runtimes_from_project(project, Some(task));
        let node = ActionNode::RunTarget(runtime, target.id.to_owned());

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(*index));
        }

        // Compare against touched files if provided
        if let Some(touched) = touched_files {
            if !task.is_affected(touched, self.workspace_root)? {
                trace!(
                    target: LOG_TARGET,
                    "Target {} not affected based on touched files, skipping",
                    color::target(&target.id),
                );

                return Ok(None);
            }
        }

        trace!(
            target: LOG_TARGET,
            "Adding {} to graph",
            color::muted(node.label())
        );

        // We should install deps & sync projects *before* running targets
        let install_deps_index = self.install_deps(project, Some(task))?;
        let sync_project_index = self.sync_project(project)?;
        let index = self.insert_node(&node);

        self.graph.add_edge(index, install_deps_index, ());
        self.graph.add_edge(index, sync_project_index, ());

        // And we also need to wait on all dependent targets
        if !task.deps.is_empty() {
            trace!(
                target: LOG_TARGET,
                "Adding dependencies {} for target {}",
                map_list(&task.deps, |f| color::symbol(f)),
                color::target(target),
            );

            for dep_index in self.run_target_task_dependencies(task, touched_files)? {
                self.graph.add_edge(index, dep_index, ());
            }
        }

        Ok(Some(index))
    }

    pub fn run_target_task_dependencies(
        &mut self,
        task: &Task,
        touched_files: Option<&TouchedFilePaths>,
    ) -> Result<Vec<NodeIndex>, DepGraphError> {
        let parallel = task.options.run_deps_in_parallel;
        let mut indexes = vec![];
        let mut previous_target_index = None;

        for dep_target in &task.deps {
            let (_, dep_indexes) = self.run_target(dep_target, touched_files)?;

            for dep_index in dep_indexes {
                // When parallel, parent depends on child
                if parallel {
                    indexes.push(dep_index);

                    // When serial, next child depends on previous child
                } else if let Some(prev) = previous_target_index {
                    self.graph.add_edge(dep_index, prev, ());
                }

                previous_target_index = Some(dep_index);
            }
        }

        if !parallel {
            indexes.push(previous_target_index.unwrap());
        }

        Ok(indexes)
    }

    pub fn run_targets_by_id(
        &mut self,
        target_ids: &[String],
        touched_files: Option<&TouchedFilePaths>,
    ) -> Result<Vec<Target>, DepGraphError> {
        let mut qualified_targets = vec![];

        for target_id in target_ids {
            let (targets, _) = self.run_target(Target::parse(target_id)?, touched_files)?;

            qualified_targets.extend(targets);
        }

        Ok(qualified_targets)
    }

    pub fn setup_tool(&mut self, runtime: &Runtime) -> NodeIndex {
        let node = ActionNode::SetupTool(runtime.clone());

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        trace!(
            target: LOG_TARGET,
            "Adding {} to graph",
            color::muted(node.label())
        );

        self.insert_node(&node)
    }

    pub fn sync_project(&mut self, project: &Project) -> Result<NodeIndex, DepGraphError> {
        let (runtime, _) = self.get_runtimes_from_project(project, None);
        let node = ActionNode::SyncProject(runtime.clone(), project.id.to_owned());

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(*index);
        }

        trace!(
            target: LOG_TARGET,
            "Adding {} to graph",
            color::muted(node.label())
        );

        // Syncing depends on the language's tool to be installed
        let setup_tool_index = self.setup_tool(&runtime);
        let index = self.insert_node(&node);

        self.graph.add_edge(index, setup_tool_index, ());

        // And we should also depend on other projects
        for dep_project_id in self.project_graph.get_dependencies_of(project)? {
            let dep_project = self.project_graph.get(&dep_project_id)?;
            let dep_index = self.sync_project(dep_project)?;

            if index != dep_index {
                self.graph.add_edge(index, dep_index, ());
            }
        }

        Ok(index)
    }

    // PRIVATE

    fn insert_node(&mut self, node: &ActionNode) -> NodeIndex {
        let index = self.graph.add_node(node.to_owned());
        self.indices.insert(node.to_owned(), index);
        index
    }
}
