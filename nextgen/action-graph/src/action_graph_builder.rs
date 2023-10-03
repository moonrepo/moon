use crate::action_graph::ActionGraph;
use crate::action_node::ActionNode;
use moon_common::{color, path::WorkspaceRelativePathBuf};
use moon_platform::{PlatformManager, Runtime};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_query::{build_query, Criteria};
use moon_task::{Target, TargetError, TargetLocator, TargetScope, Task};
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use tracing::{debug, trace};

type TouchedFilePaths = FxHashSet<WorkspaceRelativePathBuf>;

// TODO: run task dependents

pub struct ActionGraphBuilder<'app> {
    all_query: Option<Criteria>,
    graph: StableGraph<ActionNode, ()>,
    indices: FxHashMap<ActionNode, NodeIndex>,
    platform_manager: &'app PlatformManager,
    project_graph: &'app ProjectGraph,
}

impl<'app> ActionGraphBuilder<'app> {
    pub fn new(project_graph: &'app ProjectGraph) -> miette::Result<Self> {
        ActionGraphBuilder::with_platforms(PlatformManager::read(), project_graph)
    }

    pub fn with_platforms(
        platform_manager: &'app PlatformManager,
        project_graph: &'app ProjectGraph,
    ) -> miette::Result<Self> {
        Ok(ActionGraphBuilder {
            all_query: None,
            graph: StableGraph::new(),
            indices: FxHashMap::default(),
            platform_manager,
            project_graph,
        })
    }

    pub fn build(self) -> miette::Result<ActionGraph> {
        Ok(ActionGraph::new(self.graph, self.indices))
    }

    pub fn get_index_from_node(&self, node: &ActionNode) -> Option<&NodeIndex> {
        self.indices.get(node)
    }

    pub fn get_runtime(
        &mut self,
        project: &Project,
        task: Option<&Task>,
        allow_override: bool,
    ) -> Runtime {
        if let Some(platform) = self.platform_manager.find(|p| match task {
            Some(task) => p.matches(&task.platform, None),
            None => p.matches(&project.language.clone().into(), None),
        }) {
            return platform.get_runtime_from_config(if allow_override {
                Some(&project.config)
            } else {
                None
            });
        }

        Runtime::system()
    }

    pub fn set_query(&mut self, input: &str) -> miette::Result<()> {
        self.all_query = Some(build_query(input)?);

        Ok(())
    }

    // ACTIONS

    pub fn install_deps(
        &mut self,
        runtime: &Runtime,
        project: &Project,
    ) -> miette::Result<NodeIndex> {
        let mut in_project = false;

        // If project is NOT in the package manager workspace, then we should
        // install dependencies in the project, not the workspace root.
        if let Ok(platform) = self.platform_manager.get(project.language.clone()) {
            if !platform.is_project_in_dependency_workspace(project.source.as_str())? {
                in_project = true;

                debug!(
                    "Project {} not within dependency manager workspace, dependencies will be installed within the project instead of the root",
                    color::id(&project.id),
                );
            }
        }

        let node = if in_project {
            ActionNode::InstallProjectDeps {
                project: project.id.to_owned(),
                runtime: runtime.to_owned(),
            }
        } else {
            ActionNode::InstallDeps {
                runtime: self.get_runtime(project, None, false),
            }
        };

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(*index);
        }

        // Before we install deps, we must ensure the language has been installed
        let setup_tool_index = self.setup_tool(node.get_runtime());
        let index = self.insert_node(node);

        self.graph.add_edge(index, setup_tool_index, ());

        Ok(index)
    }

    pub fn run_task(
        &mut self,
        project: &Project,
        task: &Task,
        touched_files: Option<&TouchedFilePaths>,
    ) -> miette::Result<Option<NodeIndex>> {
        let node = ActionNode::RunTask {
            interactive: task.is_interactive(),
            persistent: task.is_persistent(),
            runtime: self.get_runtime(project, Some(task), true),
            target: task.target.to_owned(),
        };

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(*index));
        }

        // Compare against touched files if provided
        if let Some(touched) = touched_files {
            if !task.is_affected(touched)? {
                trace!(
                    "Task {} not affected based on touched files, skipping",
                    color::label(&task.target),
                );

                return Ok(None);
            }
        }

        // We should install deps & sync projects *before* running targets
        let install_deps_index = self.install_deps(node.get_runtime(), project)?;
        let sync_project_index = self.sync_project(project)?;
        let index = self.insert_node(node);

        self.graph.add_edge(index, install_deps_index, ());
        self.graph.add_edge(index, sync_project_index, ());

        // And we also need to create edges for task dependencies
        if !task.deps.is_empty() {
            trace!(
                deps = ?task.deps.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
                "Adding dependencies for task {}",
                color::label(&task.target),
            );

            // We don't pass touched files to dependencies, because if the parent
            // task is affected/going to run, then so should all of these!
            for dep_index in self.run_task_dependencies(task, None)? {
                self.graph.add_edge(index, dep_index, ());
            }
        }

        Ok(Some(index))
    }

    pub fn run_task_dependencies(
        &mut self,
        task: &Task,
        touched_files: Option<&TouchedFilePaths>,
    ) -> miette::Result<Vec<NodeIndex>> {
        let parallel = task.options.run_deps_in_parallel;
        let mut indices = vec![];
        let mut previous_target_index = None;

        for dep_target in &task.deps {
            let (_, dep_indices) = self.run_task_by_target(dep_target, touched_files)?;

            for dep_index in dep_indices {
                // When parallel, parent depends on child
                if parallel {
                    indices.push(dep_index);

                    // When serial, next child depends on previous child
                } else if let Some(prev) = previous_target_index {
                    self.graph.add_edge(dep_index, prev, ());
                }

                previous_target_index = Some(dep_index);
            }
        }

        if !parallel {
            indices.push(previous_target_index.unwrap());
        }

        Ok(indices)
    }

    pub fn run_task_by_target<T: AsRef<Target>>(
        &mut self,
        target: T,
        touched_files: Option<&TouchedFilePaths>,
    ) -> miette::Result<(FxHashSet<Target>, FxHashSet<NodeIndex>)> {
        let target = target.as_ref();
        let mut inserted_targets = FxHashSet::default();
        let mut inserted_indices = FxHashSet::default();

        match &target.scope {
            // :task
            TargetScope::All => {
                let mut projects = vec![];

                if let Some(all_query) = &self.all_query {
                    projects.extend(self.project_graph.query(all_query)?);
                } else {
                    projects.extend(self.project_graph.get_all()?);
                };

                for project in projects {
                    // Don't error if the task does not exist
                    if let Ok(task) = project.get_task(&target.task_id) {
                        if let Some(index) = self.run_task(&project, task, touched_files)? {
                            inserted_targets.insert(task.target.clone());
                            inserted_indices.insert(index);
                        }
                    }
                }
            }
            // ^:task
            TargetScope::Deps => {
                return Err(TargetError::NoDepsInRunContext.into());
            }
            // project:task
            TargetScope::Project(project_locator) => {
                let project = self.project_graph.get(project_locator)?;
                let task = project.get_task(&target.task_id)?;

                if let Some(index) = self.run_task(&project, task, touched_files)? {
                    inserted_targets.insert(task.target.to_owned());
                    inserted_indices.insert(index);
                }
            }
            // #tag:task
            TargetScope::Tag(tag) => {
                let projects = self
                    .project_graph
                    .query(build_query(format!("tag={}", tag))?)?;

                for project in projects {
                    // Don't error if the task does not exist
                    if let Ok(task) = project.get_task(&target.task_id) {
                        if let Some(index) = self.run_task(&project, task, touched_files)? {
                            inserted_targets.insert(task.target.clone());
                            inserted_indices.insert(index);
                        }
                    }
                }
            }
            // ~:task
            TargetScope::OwnSelf => {
                return Err(TargetError::NoSelfInRunContext.into());
            }
        };

        Ok((inserted_targets, inserted_indices))
    }

    pub fn run_task_by_target_locator<T: AsRef<TargetLocator>>(
        &mut self,
        target_locator: T,
        touched_files: Option<&TouchedFilePaths>,
    ) -> miette::Result<(FxHashSet<Target>, FxHashSet<NodeIndex>)> {
        match target_locator.as_ref() {
            TargetLocator::Qualified(target) => self.run_task_by_target(target, touched_files),
            TargetLocator::TaskFromWorkingDir(task_id) => self.run_task_by_target(
                Target::new(&self.project_graph.get_from_path(None)?.id, task_id)?,
                touched_files,
            ),
        }
    }

    pub fn setup_tool(&mut self, runtime: &Runtime) -> NodeIndex {
        let node = ActionNode::SetupTool {
            runtime: runtime.to_owned(),
        };

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        let sync_workspace_index = self.sync_workspace();
        let index = self.insert_node(node);

        self.graph.add_edge(index, sync_workspace_index, ());

        index
    }

    pub fn sync_project(&mut self, project: &Project) -> miette::Result<NodeIndex> {
        let node = ActionNode::SyncProject {
            project: project.id.clone(),
            runtime: self.get_runtime(project, None, true),
        };

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(*index);
        }

        // Syncing requires the language's tool to be installed
        let setup_tool_index = self.setup_tool(node.get_runtime());
        let index = self.insert_node(node);
        let mut reqs = vec![setup_tool_index];

        // And we should also depend on other projects
        for dep_project_id in self.project_graph.dependencies_of(project)? {
            let dep_project = self.project_graph.get(dep_project_id)?;
            let dep_project_index = self.sync_project(&dep_project)?;

            if index != dep_project_index {
                reqs.push(dep_project_index);
            }
        }

        self.link_requirements(index, reqs);

        Ok(index)
    }

    pub fn sync_workspace(&mut self) -> NodeIndex {
        let node = ActionNode::SyncWorkspace;

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        self.insert_node(node)
    }

    // PRIVATE

    fn link_requirements(&mut self, index: NodeIndex, reqs: Vec<NodeIndex>) {
        trace!(
            index = index.index(),
            requires = ?reqs,
            "Linking requirements for index"
        );

        for req in reqs {
            self.graph.add_edge(index, req, ());
        }
    }

    fn insert_node(&mut self, node: ActionNode) -> NodeIndex {
        let index = self.graph.add_node(node.clone());

        debug!(
            index = index.index(),
            "Adding {} to graph",
            color::muted_light(node.label())
        );

        self.indices.insert(node, index);

        index
    }
}
