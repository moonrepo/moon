use crate::action_graph::ActionGraph;
use miette::IntoDiagnostic;
use moon_action::{
    ActionNode, InstallDependenciesNode, InstallProjectDepsNode, InstallWorkspaceDepsNode,
    RunTaskNode, SetupEnvironmentNode, SetupToolchainLegacyNode, SetupToolchainNode,
    SyncProjectNode,
};
use moon_action_context::{ActionContext, TargetState};
use moon_affected::{AffectedTracker, DownstreamScope, UpstreamScope};
use moon_app_context::AppContext;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf, is_root_level_source};
use moon_common::{Id, color};
use moon_config::{PipelineActionSwitch, TaskDependencyConfig};
use moon_pdk_api::LocateDependenciesRootInput;
use moon_platform::{PlatformManager, Runtime, ToolchainSpec};
use moon_project::Project;
use moon_query::{Criteria, build_query};
use moon_task::{Target, TargetError, TargetLocator, TargetScope, Task};
use moon_task_args::parse_task_args;
use moon_workspace_graph::{GraphConnections, WorkspaceGraph, tasks::TaskGraphError};
use petgraph::prelude::*;
use petgraph::visit::IntoNodeReferences;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::glob::GlobSet;
use std::mem;
use std::sync::Arc;
use tracing::{debug, instrument, trace};

#[derive(Default)]
pub struct RunRequirements {
    pub ci: bool,          // Are we in a CI environment
    pub ci_check: bool,    // Check the `runInCI` option
    pub dependents: bool,  // Run dependent tasks as well
    pub interactive: bool, // Entire pipeline is interactive
}

pub struct ActionGraphBuilderOptions {
    pub install_dependencies: PipelineActionSwitch,
    pub setup_environment: PipelineActionSwitch, // TODO
    pub setup_toolchains: PipelineActionSwitch,
    pub sync_projects: PipelineActionSwitch,
    pub sync_project_dependencies: bool,
    pub sync_workspace: bool,
}

impl Default for ActionGraphBuilderOptions {
    fn default() -> Self {
        Self::new(true)
    }
}

impl ActionGraphBuilderOptions {
    pub fn new(state: bool) -> Self {
        Self {
            install_dependencies: state.into(),
            setup_environment: state.into(),
            setup_toolchains: state.into(),
            sync_projects: state.into(),
            sync_project_dependencies: state,
            sync_workspace: state,
        }
    }
}

// sync_workspace
//   - change workspace/root files
//   - change toolchain files
// sync_project
//   - change project files/manifests
//   - change toolchain files
// install_deps:
// setup_toolchain:
// run_task:
pub struct ActionGraphBuilder<'query> {
    all_query: Option<Criteria<'query>>,
    app_context: Arc<AppContext>,
    graph: DiGraph<ActionNode, ()>,
    options: ActionGraphBuilderOptions,
    platform_manager: Option<PlatformManager>,
    workspace_graph: Arc<WorkspaceGraph>,

    // Affected tracking
    affected: Option<AffectedTracker>,
    touched_files: Option<FxHashSet<WorkspaceRelativePathBuf>>,

    // Target tracking
    // initial_targets: FxHashSet<Target>,
    passthrough_targets: FxHashSet<Target>,
    primary_targets: FxHashSet<Target>,
}

impl<'query> ActionGraphBuilder<'query> {
    pub fn new(
        app_context: Arc<AppContext>,
        workspace_graph: Arc<WorkspaceGraph>,
        options: ActionGraphBuilderOptions,
    ) -> miette::Result<Self> {
        debug!("Building action graph");

        Ok(ActionGraphBuilder {
            affected: None,
            all_query: None,
            app_context,
            graph: DiGraph::new(),
            // initial_targets: FxHashSet::default(),
            options,
            passthrough_targets: FxHashSet::default(),
            platform_manager: None,
            primary_targets: FxHashSet::default(),
            touched_files: None,
            workspace_graph,
        })
    }

    pub fn build(mut self) -> (ActionContext, ActionGraph) {
        let mut context = ActionContext {
            affected: self.affected.take().map(|affected| affected.build()),
            ..ActionContext::default()
        };

        // TODO
        // if !self.initial_targets.is_empty() {
        //     context.initial_targets = mem::take(&mut self.initial_targets);
        // }

        if !self.passthrough_targets.is_empty() {
            for target in mem::take(&mut self.passthrough_targets) {
                context.set_target_state(target, TargetState::Passthrough);
            }
        }

        if !self.primary_targets.is_empty() {
            context.primary_targets = mem::take(&mut self.primary_targets);
        }

        if let Some(files) = self.touched_files.take() {
            context.touched_files = files.to_owned();
        }

        (context, ActionGraph::new(self.graph))
    }

    pub fn get_runtime(
        &self,
        project: &Project,
        toolchain_id: &Id,
        allow_override: bool,
    ) -> Runtime {
        if let Ok(platform) = self
            .platform_manager
            .as_ref()
            .unwrap()
            .get_by_toolchain(toolchain_id)
        {
            return platform.get_runtime_from_config(if allow_override {
                Some(&project.config)
            } else {
                None
            });
        }

        Runtime::system()
    }

    pub fn get_spec(
        &self,
        project: &Project,
        toolchain_id: &Id,
        allow_override: bool,
    ) -> Option<ToolchainSpec> {
        if let Some(config) = project.config.toolchain.plugins.get(toolchain_id) {
            if !config.is_enabled() {
                return None;
            }

            if allow_override {
                if let Some(version) = config.get_version() {
                    return Some(ToolchainSpec::new_override(
                        toolchain_id.to_owned(),
                        version.to_owned(),
                    ));
                }
            }
        }

        if let Some(config) = self.app_context.toolchain_config.plugins.get(toolchain_id) {
            return Some(match &config.version {
                Some(version) => ToolchainSpec::new(toolchain_id.to_owned(), version.to_owned()),
                None => ToolchainSpec::new_global(toolchain_id.to_owned()),
            });
        }

        None
    }

    pub fn set_affected(&mut self) -> miette::Result<()> {
        if self.affected.is_none() {
            self.affected = Some(AffectedTracker::new(
                Arc::clone(&self.workspace_graph),
                self.touched_files
                    .as_ref()
                    .expect("Touched files are required for affected tracking.")
                    .to_owned(),
            ));
        }

        Ok(())
    }

    pub fn set_platform_manager(&mut self, manager: PlatformManager) -> miette::Result<()> {
        self.platform_manager = Some(manager);

        Ok(())
    }

    pub fn set_query(&mut self, input: &'query str) -> miette::Result<()> {
        self.all_query = Some(build_query(input)?);

        Ok(())
    }

    pub fn set_touched_files(
        &mut self,
        touched_files: FxHashSet<WorkspaceRelativePathBuf>,
    ) -> miette::Result<()> {
        self.touched_files = Some(touched_files);

        Ok(())
    }

    pub fn track_affected(
        &mut self,
        upstream: UpstreamScope,
        downstream: DownstreamScope,
        ci_check: bool,
    ) -> miette::Result<()> {
        // If we require dependents, then we must load all projects into the
        // graph so that the edges are created!
        if downstream != DownstreamScope::None {
            debug!("Force loading all projects and tasks to determine relationships");

            self.workspace_graph.get_projects()?;
            self.workspace_graph.get_tasks_with_internal()?;
        }

        self.set_affected()?;

        if let Some(affected) = self.affected.as_mut() {
            affected.set_ci_check(ci_check);
            affected.with_scopes(upstream, downstream);
            affected.track_projects()?;
            affected.track_tasks()?;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn install_dependencies_legacy(
        &mut self,
        runtime: &Runtime,
        project: &Project,
        has_bun_and_node: bool,
    ) -> miette::Result<Option<NodeIndex>> {
        let setup_toolchain_index = self.setup_toolchain_legacy(runtime).await?;

        if !self
            .options
            .install_dependencies
            .is_enabled(&runtime.toolchain)
            || runtime.is_system()
        {
            return Ok(setup_toolchain_index);
        }

        // If Bun and Node.js are enabled, they will both attempt to install
        // dependencies in the target root. We need to avoid this problem,
        // so always prefer Node.js instead. Revisit in the future.
        if has_bun_and_node && runtime.toolchain == "bun" {
            debug!(
                "Already installing dependencies with node, skipping a conflicting install from bun"
            );

            return Ok(setup_toolchain_index);
        }

        let platform = self
            .platform_manager
            .as_ref()
            .unwrap()
            .get_by_toolchain(&runtime.toolchain)?;

        let packages_root = platform.find_dependency_workspace_root(project.source.as_str())?;
        let mut in_project = false;

        // If project is NOT in the package manager workspace, then we should
        // install dependencies in the project, not the workspace root.
        if !platform.is_project_in_dependency_workspace(&packages_root, project.source.as_str())? {
            in_project = true;

            debug!(
                "Project {} is not within the dependency manager workspace, dependencies will be installed within the project instead of the root",
                color::id(&project.id),
            );
        }

        let index = self.insert_node(if in_project {
            ActionNode::install_project_deps(InstallProjectDepsNode {
                project_id: project.id.to_owned(),
                runtime: runtime.to_owned(),
            })
        } else {
            ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                runtime: runtime.to_owned(),
                root: packages_root,
            })
        });

        // Before we install deps, we must ensure the language has been installed
        self.link_requirements(index, vec![setup_toolchain_index]);

        Ok(Some(index))
    }

    #[instrument(skip_all)]
    pub async fn install_dependencies(
        &mut self,
        spec: &ToolchainSpec,
        project: &Project,
    ) -> miette::Result<Option<NodeIndex>> {
        let setup_toolchain_index = self.setup_toolchain(spec).await?;

        // Explicitly disabled
        if !self.options.install_dependencies.is_enabled(&spec.id) || spec.is_system() {
            return Ok(setup_toolchain_index);
        }

        let registry = &self.app_context.toolchain_registry;
        let toolchain = registry.load(&spec.id).await?;

        // Toolchain does not support this action, so skip and fall through
        if !toolchain.supports_tier_2().await {
            return Ok(setup_toolchain_index);
        }

        let output = toolchain
            .locate_dependencies_root(LocateDependenciesRootInput {
                context: registry.create_context(),
                starting_dir: toolchain.to_virtual_path(&project.root),
            })
            .await?;

        // Only insert this action if a root was located
        if let Some(root) = output.root {
            let abs_root = toolchain.from_virtual_path(root.any_path());
            let rel_root = abs_root
                .relative_to(&self.app_context.workspace_root)
                .into_diagnostic()?;

            // Determine if we're in the dependencies workspace
            let in_project = project.root == abs_root;
            let in_workspace = if let Some(globs) = output.members {
                if in_project {
                    true // Root always in the workspace
                } else {
                    GlobSet::new(&globs)?.matches(project.source.as_str())
                }
            } else {
                true
            };

            // If not in the dependencies workspace (if there is one),
            // or is a stand-alone project with its own lockfile,
            // we must extract the project ID and source (root)
            let (project_id, root) =
                if !in_workspace || in_project && !is_root_level_source(&project.source) {
                    (Some(project.id.clone()), project.source.clone())
                } else {
                    (None, rel_root)
                };

            let setup_env = ActionNode::setup_environment(SetupEnvironmentNode {
                project_id: project_id.clone(),
                root: root.clone(),
                toolchain_id: spec.id.clone(),
            });

            let install_deps = ActionNode::install_dependencies(InstallDependenciesNode {
                project_id,
                root,
                toolchain_id: spec.id.clone(),
            });

            // We need to conditionally create nodes and edges based on what
            // APIs have been implemented by the plugin
            let has_install_deps = toolchain.has_func("install_dependencies").await;
            let has_setup_env = toolchain.has_func("setup_environment").await;

            let index = match (has_install_deps, has_setup_env) {
                (true, true) => {
                    let setup_env_index = self.insert_node(setup_env);
                    let install_deps_index = self.insert_node(install_deps);

                    self.link_requirements(install_deps_index, vec![Some(setup_env_index)]);
                    self.link_requirements(setup_env_index, vec![setup_toolchain_index]);

                    Some(install_deps_index)
                }
                (true, false) => {
                    let install_deps_index = self.insert_node(install_deps);

                    self.link_requirements(install_deps_index, vec![setup_toolchain_index]);

                    Some(install_deps_index)
                }
                (false, true) => {
                    let setup_env_index = self.insert_node(setup_env);

                    self.link_requirements(setup_env_index, vec![setup_toolchain_index]);

                    Some(setup_env_index)
                }
                (false, false) => setup_toolchain_index,
            };

            return Ok(index);
        }

        Ok(setup_toolchain_index)
    }

    pub async fn run_task(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
    ) -> miette::Result<Option<NodeIndex>> {
        if let Some(index) = self.internal_run_task(&task, reqs, None).await? {
            // Only track primary targets at the top-level run methods,
            // as these are explicitly called by pipeline consumers!
            self.primary_targets.insert(task.target.clone());

            return Ok(Some(index));
        }

        Ok(None)
    }

    pub async fn run_task_by_target<T: AsRef<Target>>(
        &mut self,
        target: T,
        reqs: &RunRequirements,
    ) -> miette::Result<FxHashSet<NodeIndex>> {
        let target = target.as_ref();
        let mut indexes = FxHashSet::default();

        for task in self
            .internal_resolve_tasks_from_target(target, false)
            .await?
        {
            if let Some(index) = self.run_task(&task, reqs).await? {
                indexes.insert(index);
            }
        }

        Ok(indexes)
    }

    pub async fn run_task_by_target_locator<T: AsRef<TargetLocator>>(
        &mut self,
        locator: T,
        reqs: &RunRequirements,
    ) -> miette::Result<FxHashSet<NodeIndex>> {
        let locator = locator.as_ref();
        let mut indexes = FxHashSet::default();

        for task in self
            .internal_resolve_tasks_from_target_locator(locator, false)
            .await?
        {
            if let Some(index) = self.run_task(&task, reqs).await? {
                indexes.insert(index);
            }
        }

        Ok(indexes)
    }

    pub async fn run_task_dependencies(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
    ) -> miette::Result<Vec<Option<NodeIndex>>> {
        let parallel = task.options.run_deps_in_parallel;
        let mut indexes: Vec<Option<NodeIndex>> = vec![];
        let mut previous_target_index: Option<NodeIndex> = None;

        for dep in &task.deps {
            for dep_task in self
                .internal_resolve_tasks_from_target(&dep.target, true)
                .await?
            {
                if let Some(dep_index) = self.internal_run_task(&dep_task, reqs, Some(dep)).await? {
                    // When parallel, parent depends on child
                    if parallel {
                        indexes.push(Some(dep_index));
                    }
                    // When serial, next child depends on previous child
                    else {
                        self.link_requirements(dep_index, vec![previous_target_index]);
                    }

                    previous_target_index = Some(dep_index);
                }
            }
        }

        if !parallel {
            indexes.push(previous_target_index);
        }

        Ok(indexes)
    }

    pub async fn run_task_dependents(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
    ) -> miette::Result<Vec<Option<NodeIndex>>> {
        let mut indexes = vec![];

        for dep_target in self.workspace_graph.tasks.dependents_of(task) {
            for dep_task in self
                .internal_resolve_tasks_from_target(&dep_target, true)
                .await?
            {
                indexes.push(self.internal_run_task(&dep_task, reqs, None).await?);
            }
        }

        Ok(indexes)
    }

    #[instrument(skip_all)]
    async fn internal_resolve_tasks_from_target(
        &mut self,
        target: &Target,
        allow_internal: bool,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let mut tasks = vec![];

        match &target.scope {
            // :task
            TargetScope::All => {
                let mut projects = vec![];

                if let Some(all_query) = &self.all_query {
                    projects.extend(self.workspace_graph.query_projects(all_query)?);
                } else {
                    projects.extend(self.workspace_graph.get_projects()?);
                };

                for project in projects {
                    // Don't error if the task does not exist
                    if let Ok(task) = self
                        .workspace_graph
                        .get_task_from_project(&project.id, &target.task_id)
                    {
                        if !allow_internal && task.is_internal() {
                            continue;
                        }

                        tasks.push(task);
                    }
                }
            }
            // ^:task
            TargetScope::Deps => {
                return Err(TargetError::NoDepsInRunContext.into());
            }
            // project:task
            TargetScope::Project(project_id) => {
                let task = self
                    .workspace_graph
                    .get_task_from_project(project_id, &target.task_id)?;

                // Don't allow internal tasks to be ran
                if !allow_internal && task.is_internal() {
                    return Err(TaskGraphError::UnconfiguredTarget(task.target.clone()).into());
                }

                tasks.push(task);
            }
            // #tag:task
            TargetScope::Tag(tag) => {
                let projects = self
                    .workspace_graph
                    .query_projects(build_query(format!("tag={}", tag).as_str())?)?;

                for project in projects {
                    // Don't error if the task does not exist
                    if let Ok(task) = self
                        .workspace_graph
                        .get_task_from_project(&project.id, &target.task_id)
                    {
                        if !allow_internal && task.is_internal() {
                            continue;
                        }

                        tasks.push(task);
                    }
                }
            }
            // ~:task
            TargetScope::OwnSelf => {
                return Err(TargetError::NoSelfInRunContext.into());
            }
        };

        Ok(tasks)
    }

    #[instrument(skip_all)]
    async fn internal_resolve_tasks_from_target_locator(
        &mut self,
        locator: &TargetLocator,
        allow_internal: bool,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let mut tasks = vec![];

        match locator {
            TargetLocator::GlobMatch {
                project_glob,
                task_glob,
                scope,
                ..
            } => {
                let mut is_all = false;
                let mut do_query = false;
                let mut projects = vec![];

                // Query for all applicable projects first since we can't
                // query projects + tasks at the same time
                if let Some(glob) = project_glob {
                    let query = if let Some(tag_glob) = glob.strip_prefix('#') {
                        format!("tag~{tag_glob}")
                    } else {
                        format!("project~{glob}")
                    };

                    projects = self.workspace_graph.query_projects(build_query(&query)?)?;
                    do_query = !projects.is_empty();
                } else {
                    match scope {
                        Some(TargetScope::All) => {
                            is_all = true;
                            do_query = true;
                        }
                        _ => {
                            // Don't query for the other scopes,
                            // since they're not valid from the run context
                        }
                    };
                }

                // Then query for all tasks within the queried projects
                if do_query {
                    let mut query = format!("task~{task_glob}");

                    if !is_all {
                        query = format!(
                            "project=[{}] && {query}",
                            projects
                                .into_iter()
                                .map(|project| project.id.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                    }

                    for task in self.workspace_graph.query_tasks(build_query(&query)?)? {
                        if !allow_internal && task.is_internal() {
                            continue;
                        }

                        tasks.push(task);
                    }
                }
            }
            TargetLocator::Qualified(target) => {
                let target = if target.scope == TargetScope::OwnSelf {
                    Target::new(
                        &self.workspace_graph.get_project_from_path(None)?.id,
                        &target.task_id,
                    )?
                } else {
                    target.to_owned()
                };

                tasks.extend(
                    self.internal_resolve_tasks_from_target(&target, allow_internal)
                        .await?,
                );
            }
            TargetLocator::TaskFromWorkingDir(task_id) => {
                let target = Target::new(
                    &self.workspace_graph.get_project_from_path(None)?.id,
                    task_id,
                )?;

                tasks.extend(
                    self.internal_resolve_tasks_from_target(&target, allow_internal)
                        .await?,
                );
            }
        };

        Ok(tasks)
    }

    #[instrument(skip_all)]
    async fn internal_run_task(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
        config: Option<&TaskDependencyConfig>,
    ) -> miette::Result<Option<NodeIndex>> {
        let project = self
            .workspace_graph
            .get_project(task.target.get_project_id().unwrap())?;

        let child_reqs = RunRequirements {
            ci: reqs.ci,
            ci_check: reqs.ci_check,
            dependents: false,
            interactive: reqs.interactive,
        };

        // Abort early if not affected
        if let Some(affected) = &mut self.affected {
            if !affected.is_task_marked(task) {
                return Ok(None);
            }
        }

        // These tasks shouldn't actually run, so filter them out
        if self.passthrough_targets.contains(&task.target) {
            trace!(
                task_target = task.target.as_str(),
                "Not running task {} because it has been marked as passthrough",
                color::id(&task.target.id),
            );

            return Ok(None);
        }

        // Only apply checks when requested. This applies to `moon ci`,
        // but not `moon run`, since the latter should be able to
        // manually run local tasks in CI (deploys, etc).
        if reqs.ci && reqs.ci_check && !task.should_run_in_ci() {
            self.passthrough_targets.insert(task.target.clone());

            debug!(
                task_target = task.target.as_str(),
                "Not running task {} because {} is false",
                color::id(&task.target.id),
                color::property("runInCI"),
            );

            // Dependents may still want to run though!
            if reqs.dependents {
                Box::pin(self.run_task_dependents(task, &child_reqs)).await?;
            }

            return Ok(None);
        }

        // Create the node
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
            priority: task.options.priority.get_level(),
            target: task.target.to_owned(),
            id: None,
        });

        // Check if the node exists to avoid all the overhead below
        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(index));
        }

        // Create initial edges
        let mut edges = vec![self.sync_project(&project).await?];
        let has_bun_and_node = task.toolchains.iter().any(|tc| tc == "node")
            && task.toolchains.iter().any(|tc| tc == "bun");

        for toolchain_id in &task.toolchains {
            if self.app_context.toolchain_config.is_plugin(toolchain_id) {
                if let Some(spec) = self.get_spec(&project, toolchain_id, true) {
                    edges.push(self.install_dependencies(&spec, &project).await?);
                }
            } else {
                let runtime = self.get_runtime(&project, toolchain_id, true);

                edges.push(
                    self.install_dependencies_legacy(&runtime, &project, has_bun_and_node)
                        .await?,
                );
            }
        }

        // Insert and then link edges
        let index = self.insert_node(node);

        if !task.deps.is_empty() {
            edges.extend(Box::pin(self.run_task_dependencies(task, &child_reqs)).await?);
        }

        self.link_requirements(index, edges);

        // And possibly dependents
        if reqs.dependents {
            Box::pin(self.run_task_dependents(task, &child_reqs)).await?;
        }

        Ok(Some(index))
    }

    #[instrument(skip_all)]
    pub async fn setup_toolchain_legacy(
        &mut self,
        runtime: &Runtime,
    ) -> miette::Result<Option<NodeIndex>> {
        let sync_workspace_index = self.sync_workspace().await;

        // Explicitly disabled
        if !self.options.setup_toolchains.is_enabled(&runtime.toolchain) || runtime.is_system() {
            return Ok(sync_workspace_index);
        }

        let index = self.insert_node(ActionNode::setup_toolchain_legacy(
            SetupToolchainLegacyNode {
                runtime: runtime.to_owned(),
            },
        ));

        self.link_requirements(index, vec![sync_workspace_index]);

        Ok(Some(index))
    }

    #[instrument(skip_all)]
    pub async fn setup_toolchain(
        &mut self,
        spec: &ToolchainSpec,
    ) -> miette::Result<Option<NodeIndex>> {
        let sync_workspace_index = self.sync_workspace().await;

        // Explicitly disabled
        if !self.options.setup_toolchains.is_enabled(&spec.id) || spec.is_system() {
            return Ok(sync_workspace_index);
        }

        let toolchain = self.app_context.toolchain_registry.load(&spec.id).await?;

        // Toolchain does not support tier 3
        if !toolchain.supports_tier_3().await {
            return Ok(sync_workspace_index);
        }

        let index = self.insert_node(ActionNode::setup_toolchain(SetupToolchainNode {
            spec: spec.to_owned(),
        }));

        self.link_requirements(index, vec![sync_workspace_index]);

        Ok(Some(index))
    }

    #[instrument(skip_all)]
    pub async fn sync_project(&mut self, project: &Project) -> miette::Result<Option<NodeIndex>> {
        self.internal_sync_project(project, &mut FxHashSet::default())
            .await
    }

    async fn internal_sync_project(
        &mut self,
        project: &Project,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<Option<NodeIndex>> {
        let sync_workspace_index = self.sync_workspace().await;

        // Explicitly disabled
        if !self.options.sync_projects.is_enabled(&project.id) {
            return Ok(sync_workspace_index);
        }

        // Abort early if not affected
        if let Some(affected) = &mut self.affected {
            if !affected.is_project_marked(project) {
                return Ok(None);
            }
        }

        cycle.insert(project.id.clone());

        let mut edges = vec![sync_workspace_index];
        let index = self.insert_node(ActionNode::sync_project(SyncProjectNode {
            project_id: project.id.clone(),
        }));

        // And we should also depend on other projects
        if self.options.sync_project_dependencies {
            for dep_project_id in self.workspace_graph.projects.dependencies_of(project) {
                if cycle.contains(&dep_project_id) {
                    continue;
                }

                let dep_project = self.workspace_graph.get_project(&dep_project_id)?;

                if let Some(dep_project_index) =
                    Box::pin(self.internal_sync_project(&dep_project, cycle)).await?
                {
                    if index != dep_project_index {
                        edges.push(Some(dep_project_index));
                    }
                }
            }
        }

        if !edges.is_empty() {
            self.link_requirements(index, edges);
        }

        Ok(Some(index))
    }

    pub async fn sync_workspace(&mut self) -> Option<NodeIndex> {
        if !self.options.sync_workspace {
            return None;
        }

        Some(self.insert_node(ActionNode::sync_workspace()))
    }

    // PRIVATE

    fn get_index_from_node(&self, node: &ActionNode) -> Option<NodeIndex> {
        self.graph
            .node_references()
            .find(|(_, n)| *n == node)
            .map(|(i, _)| i)
    }

    fn link_requirements(&mut self, index: NodeIndex, edges: Vec<Option<NodeIndex>>) {
        let edges = edges.into_iter().flatten().collect::<Vec<_>>();

        if edges.is_empty() {
            return;
        }

        trace!(
            index = index.index(),
            requires = ?edges.iter().map(|edge| edge.index()).collect::<Vec<_>>(),
            "Linking requirements for index"
        );

        for edge in edges {
            // Use `update_edge` instead of `add_edge` as it avoids
            // duplicate edges from being inserted
            self.graph.update_edge(index, edge, ());
        }
    }

    fn insert_node(&mut self, node: ActionNode) -> NodeIndex {
        if let Some(index) = self.get_index_from_node(&node) {
            return index;
        }

        let label = node.label();
        let index = self.graph.add_node(node);

        debug!(
            index = index.index(),
            "Adding {} to graph",
            color::muted_light(label)
        );

        index
    }
}

#[cfg(debug_assertions)]
impl ActionGraphBuilder<'_> {
    pub fn mock_affected(&mut self, mut op: impl FnMut(&mut AffectedTracker)) {
        if let Some(affected) = self.affected.as_mut() {
            op(affected);
        }
    }
}
