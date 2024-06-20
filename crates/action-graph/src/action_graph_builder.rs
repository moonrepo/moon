use crate::action_graph::ActionGraph;
use moon_action::{
    ActionNode, InstallProjectDepsNode, InstallWorkspaceDepsNode, RunTaskNode, SetupToolchainNode,
    SyncProjectNode,
};
use moon_common::Id;
use moon_common::{color, path::WorkspaceRelativePathBuf};
use moon_config::{PlatformType, TaskDependencyConfig};
use moon_platform::{PlatformManager, Runtime};
use moon_project::{Project, ProjectError};
use moon_project_graph::ProjectGraph;
use moon_query::{build_query, Criteria};
use moon_task::{parse_task_args, Target, TargetError, TargetLocator, TargetScope, Task};
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use tracing::{debug, instrument, trace};

type TouchedFilePaths = FxHashSet<WorkspaceRelativePathBuf>;

#[derive(Default)]
pub struct RunRequirements<'app> {
    pub ci: bool,         // Are we in a CI environment
    pub ci_check: bool,   // Check the `runInCI` option
    pub dependents: bool, // Run dependent tasks as well
    pub initial_locators: Vec<&'app TargetLocator>,
    pub resolved_locators: Vec<TargetLocator>,
    pub interactive: bool,
    pub touched_files: Option<&'app TouchedFilePaths>,
}

impl<'app> RunRequirements<'app> {
    pub fn has_target(&self, target: &Target) -> bool {
        self.resolved_locators.iter().any(|loc| loc == target)
            || self.initial_locators.iter().any(|loc| *loc == target)
    }
}

pub struct ActionGraphBuilder<'app> {
    all_query: Option<Criteria<'app>>,
    graph: DiGraph<ActionNode, ()>,
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
        debug!("Building action graph");

        Ok(ActionGraphBuilder {
            all_query: None,
            graph: DiGraph::new(),
            indices: FxHashMap::default(),
            platform_manager,
            project_graph,
        })
    }

    pub fn build(self) -> miette::Result<ActionGraph> {
        Ok(ActionGraph::new(self.graph))
    }

    pub fn get_index_from_node(&self, node: &ActionNode) -> Option<&NodeIndex> {
        self.indices.get(node)
    }

    pub fn get_runtime(
        &self,
        project: &Project,
        platform_type: PlatformType,
        allow_override: bool,
    ) -> Runtime {
        if let Some(platform) = self
            .platform_manager
            .find(|p| p.get_type() == platform_type)
        {
            return platform.get_runtime_from_config(if allow_override {
                Some(&project.config)
            } else {
                None
            });
        }

        Runtime::system()
    }

    pub fn set_query(&mut self, input: &'app str) -> miette::Result<()> {
        self.all_query = Some(build_query(input)?);

        Ok(())
    }

    // ACTIONS

    #[instrument(skip_all)]
    pub fn install_deps(
        &mut self,
        project: &Project,
        task: Option<&Task>,
    ) -> miette::Result<Option<NodeIndex>> {
        let mut in_project = false;
        let mut platform_type = task.map(|t| t.platform).unwrap_or_else(|| project.platform);

        // If project is NOT in the package manager workspace, then we should
        // install dependencies in the project, not the workspace root.
        if let Ok(platform) = self.platform_manager.get(platform_type) {
            if !platform.is_project_in_dependency_workspace(project.source.as_str())? {
                in_project = true;

                debug!(
                    "Project {} is not within the dependency manager workspace, dependencies will be installed within the project instead of the root",
                    color::id(&project.id),
                );
            }
        }

        // If Bun and Node.js are enabled, they will both attempt to install
        // dependencies in the target root. We need to avoid this problem,
        // so always prefer Node.js instead. Revisit in the future.
        if matches!(platform_type, PlatformType::Bun)
            && self
                .platform_manager
                .enabled()
                .any(|enabled_platform| matches!(enabled_platform, PlatformType::Node))
        {
            debug!(
                "Already installing dependencies with {}, skipping a conflicting install from {}",
                PlatformType::Node,
                platform_type,
            );

            platform_type = PlatformType::Node;
        }

        let node = if in_project {
            ActionNode::install_project_deps(InstallProjectDepsNode {
                project: project.id.to_owned(),
                runtime: self.get_runtime(project, platform_type, true),
            })
        } else {
            ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                runtime: self.get_runtime(project, platform_type, false),
            })
        };

        if node.get_runtime().platform.is_system() {
            return Ok(None);
        }

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(*index));
        }

        // Before we install deps, we must ensure the language has been installed
        let setup_tool_index = self.setup_toolchain(node.get_runtime());
        let index = self.insert_node(node);

        self.link_requirements(index, vec![setup_tool_index]);

        Ok(Some(index))
    }

    pub fn run_task(
        &mut self,
        project: &Project,
        task: &Task,
        reqs: &RunRequirements<'app>,
    ) -> miette::Result<Option<NodeIndex>> {
        self.run_task_with_config(project, task, reqs, None)
    }

    #[instrument(skip_all)]
    pub fn run_task_with_config(
        &mut self,
        project: &Project,
        task: &Task,
        reqs: &RunRequirements<'app>,
        config: Option<&TaskDependencyConfig>,
    ) -> miette::Result<Option<NodeIndex>> {
        // Only apply checks when requested. This applies to `moon ci`,
        // but not `moon run`, since the latter should be able to
        // manually run local tasks in CI (deploys, etc).
        if reqs.ci && reqs.ci_check && !task.should_run_in_ci() {
            debug!(
                task = task.target.as_str(),
                "Not running task {} because {} is false",
                color::label(&task.target.id),
                color::property("runInCI"),
            );

            return Ok(None);
        }

        let mut args = vec![];
        let mut env = FxHashMap::default();

        if let Some(config) = config {
            args.extend(parse_task_args(&config.args)?);
            env.extend(config.env.clone());
        }

        let node = ActionNode::run_task(RunTaskNode {
            args,
            env,
            interactive: task.is_interactive() || reqs.interactive,
            persistent: task.is_persistent(),
            runtime: self.get_runtime(project, task.platform, true),
            target: task.target.to_owned(),
            timeout: task.options.timeout,
            id: None,
        });

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(*index));
        }

        // Compare against touched files if provided
        if let Some(touched) = &reqs.touched_files {
            if !task.is_affected(touched)? {
                return Ok(None);
            }
        }

        // We should install deps & sync projects *before* running targets
        let mut edges = vec![];

        if let Some(install_deps_index) = self.install_deps(project, Some(task))? {
            edges.push(install_deps_index);
        }

        edges.push(self.sync_project(project)?);

        let index = self.insert_node(node);

        // And we also need to create edges for task dependencies
        if !task.deps.is_empty() {
            trace!(
                task = task.target.as_str(),
                deps = ?task.deps.iter().map(|d| d.target.as_str()).collect::<Vec<_>>(),
                "Linking dependencies for task",
            );

            edges.extend(self.run_task_dependencies(task)?);
        }

        self.link_requirements(index, edges);

        // And possibly dependents
        if reqs.dependents {
            self.run_task_dependents(task, reqs)?;
        }

        Ok(Some(index))
    }

    // We don't pass touched files to dependencies, because if the parent
    // task is affected/going to run, then so should all of these!
    #[instrument(skip_all)]
    pub fn run_task_dependencies(&mut self, task: &Task) -> miette::Result<Vec<NodeIndex>> {
        let parallel = task.options.run_deps_in_parallel;
        let reqs = RunRequirements::default();
        let mut indices = vec![];
        let mut previous_target_index = None;

        for dep in &task.deps {
            let (_, dep_indices) =
                self.run_task_by_target_with_config(&dep.target, &reqs, Some(dep))?;

            for dep_index in dep_indices {
                // When parallel, parent depends on child
                if parallel {
                    indices.push(dep_index);

                    // When serial, next child depends on previous child
                } else if let Some(prev) = previous_target_index {
                    self.link_requirements(dep_index, vec![prev]);
                }

                previous_target_index = Some(dep_index);
            }
        }

        if !parallel {
            indices.push(previous_target_index.unwrap());
        }

        Ok(indices)
    }

    // This is costly, is there a better way to do this?
    #[instrument(skip_all)]
    pub fn run_task_dependents(
        &mut self,
        task: &Task,
        parent_reqs: &RunRequirements<'app>,
    ) -> miette::Result<Vec<NodeIndex>> {
        let mut indices = vec![];

        // Create a new requirements object as we only want direct
        // dependents, and shouldn't recursively create.
        let reqs = RunRequirements::default();

        if let TargetScope::Project(project_locator) = &task.target.scope {
            let mut projects_to_build = vec![];

            // From self project
            let self_project = self.project_graph.get(project_locator)?;

            projects_to_build.push(self_project.clone());

            // From other projects
            for dependent_id in self.project_graph.dependents_of(&self_project)? {
                projects_to_build.push(self.project_graph.get(dependent_id)?);
            }

            for project in projects_to_build {
                for dep_task in project.tasks.values() {
                    // Don't skip internal tasks, since they are a dependency of the parent
                    // task, and must still run! They just can't be ran manually.
                    if dep_task.is_persistent() {
                        continue;
                    }

                    // Since these are transient tasks, we should always filter out tasks
                    // that should not run in CI, as we don't know what side-effects it
                    // will cause. This applies to both `moon ci` and `moon run`.
                    if parent_reqs.ci && !dep_task.should_run_in_ci() {
                        continue;
                    }

                    if dep_task.deps.iter().any(|dep| dep.target == task.target) {
                        if let Some(index) = self.run_task(&project, dep_task, &reqs)? {
                            indices.push(index);
                        }
                    }
                }
            }
        }

        Ok(indices)
    }

    pub fn run_task_by_target<T: AsRef<Target>>(
        &mut self,
        target: T,
        reqs: &RunRequirements<'app>,
    ) -> miette::Result<(FxHashSet<Target>, FxHashSet<NodeIndex>)> {
        self.run_task_by_target_with_config(target, reqs, None)
    }

    pub fn run_task_by_target_locator<T: AsRef<TargetLocator>>(
        &mut self,
        target_locator: T,
        reqs: &mut RunRequirements<'app>,
    ) -> miette::Result<(FxHashSet<Target>, FxHashSet<NodeIndex>)> {
        match target_locator.as_ref() {
            TargetLocator::Qualified(target) => self.run_task_by_target(target, reqs),
            TargetLocator::TaskFromWorkingDir(task_id) => {
                let project = self.project_graph.get_from_path(None)?;
                let target = Target::new(&project.id, task_id)?;

                reqs.resolved_locators
                    .push(TargetLocator::Qualified(target.clone()));

                self.run_task_by_target(target, reqs)
            }
        }
    }

    pub fn run_task_by_target_with_config<T: AsRef<Target>>(
        &mut self,
        target: T,
        reqs: &RunRequirements<'app>,
        config: Option<&TaskDependencyConfig>,
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
                        if task.is_internal() {
                            continue;
                        }

                        if let Some(index) =
                            self.run_task_with_config(&project, task, reqs, config)?
                        {
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
                let project = self.project_graph.get(project_locator.as_str())?;
                let task = project.get_task(&target.task_id)?;

                // Don't allow internal tasks to be ran
                if task.is_internal() && reqs.has_target(&task.target) {
                    return Err(ProjectError::UnknownTask {
                        task_id: task.target.task_id.clone(),
                        project_id: project.id.clone(),
                    }
                    .into());
                }

                if let Some(index) = self.run_task_with_config(&project, task, reqs, config)? {
                    inserted_targets.insert(task.target.to_owned());
                    inserted_indices.insert(index);
                }
            }
            // #tag:task
            TargetScope::Tag(tag) => {
                let projects = self
                    .project_graph
                    .query(build_query(format!("tag={}", tag).as_str())?)?;

                for project in projects {
                    // Don't error if the task does not exist
                    if let Ok(task) = project.get_task(&target.task_id) {
                        if task.is_internal() {
                            continue;
                        }

                        if let Some(index) =
                            self.run_task_with_config(&project, task, reqs, config)?
                        {
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

    #[instrument(skip_all)]
    pub fn setup_toolchain(&mut self, runtime: &Runtime) -> NodeIndex {
        let node = ActionNode::setup_toolchain(SetupToolchainNode {
            runtime: runtime.to_owned(),
        });

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        let sync_workspace_index = self.sync_workspace();
        let index = self.insert_node(node);

        self.link_requirements(index, vec![sync_workspace_index]);

        index
    }

    #[instrument(skip_all)]
    pub fn sync_project(&mut self, project: &Project) -> miette::Result<NodeIndex> {
        self.internal_sync_project(project, &mut FxHashSet::default())
    }

    fn internal_sync_project(
        &mut self,
        project: &Project,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<NodeIndex> {
        let node = ActionNode::sync_project(SyncProjectNode {
            project: project.id.clone(),
            runtime: self.get_runtime(project, project.platform, true),
        });

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(*index);
        }

        cycle.insert(project.id.clone());

        // Syncing requires the language's tool to be installed
        let setup_tool_index = self.setup_toolchain(node.get_runtime());
        let index = self.insert_node(node);
        let mut edges = vec![setup_tool_index];

        // And we should also depend on other projects
        for dep_project_id in self.project_graph.dependencies_of(project)? {
            if cycle.contains(dep_project_id) {
                continue;
            }

            let dep_project = self.project_graph.get(dep_project_id)?;
            let dep_project_index = self.internal_sync_project(&dep_project, cycle)?;

            if index != dep_project_index {
                edges.push(dep_project_index);
            }
        }

        self.link_requirements(index, edges);

        Ok(index)
    }

    pub fn sync_workspace(&mut self) -> NodeIndex {
        let node = ActionNode::sync_workspace();

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        self.insert_node(node)
    }

    // PRIVATE

    fn link_requirements(&mut self, index: NodeIndex, edges: Vec<NodeIndex>) {
        trace!(
            index = index.index(),
            requires = ?edges.iter().map(|i| i.index()).collect::<Vec<_>>(),
            "Linking requirements for index"
        );

        for edge in edges {
            self.graph.add_edge(index, edge, ());
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
