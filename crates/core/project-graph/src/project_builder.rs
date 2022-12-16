use crate::errors::ProjectGraphError;
use crate::helpers::detect_projects_with_globs;
use crate::project_graph::{GraphType, IndicesType, ProjectGraph, LOG_TARGET};
use crate::token_resolver::{TokenContext, TokenResolver};
use moon_cache::CacheEngine;
use moon_config::{
    GlobalProjectConfig, PlatformType, ProjectLanguage, ProjectsAliasesMap, ProjectsSourcesMap,
    TaskConfig, WorkspaceConfig, WorkspaceProjects,
};
use moon_logger::{color, debug, map_list, trace, Logable};
use moon_platform::PlatformManager;
use moon_project::{Project, ProjectDependency, ProjectDependencySource, ProjectError};
use moon_task::{Target, TargetError, TargetProjectScope, Task, TaskError};
use moon_utils::regex::ENV_VAR;
use moon_utils::{glob, is_ci, path};
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::mem;
use std::path::{Path, PathBuf};

pub struct ProjectGraphBuilder<'ws> {
    cache: &'ws CacheEngine,
    config: &'ws GlobalProjectConfig,
    platforms: &'ws mut PlatformManager,
    workspace_config: &'ws WorkspaceConfig,
    workspace_root: &'ws Path,

    aliases: ProjectsAliasesMap,
    graph: GraphType,
    indices: IndicesType,
    sources: ProjectsSourcesMap,
}

impl<'ws> ProjectGraphBuilder<'ws> {
    pub fn new(
        cache: &'ws CacheEngine,
        config: &'ws GlobalProjectConfig,
        platforms: &'ws mut PlatformManager,
        workspace_config: &'ws WorkspaceConfig,
        workspace_root: &'ws Path,
    ) -> Result<ProjectGraphBuilder<'ws>, ProjectGraphError> {
        debug!(target: LOG_TARGET, "Creating project graph");

        let mut graph = ProjectGraphBuilder {
            aliases: FxHashMap::default(),
            cache,
            config,
            graph: DiGraph::new(),
            indices: FxHashMap::default(),
            platforms,
            sources: FxHashMap::default(),
            workspace_config,
            workspace_root,
        };

        graph.load_sources()?;
        graph.load_aliases()?;

        Ok(graph)
    }

    pub fn build(&mut self) -> ProjectGraph {
        ProjectGraph::new(
            mem::take(&mut self.graph),
            mem::take(&mut self.indices),
            mem::take(&mut self.sources),
            mem::take(&mut self.aliases),
        )
    }

    pub fn load(&mut self, alias_or_id: &str) -> Result<&Self, ProjectGraphError> {
        self.internal_load(alias_or_id)?;

        Ok(self)
    }

    pub fn load_all(&mut self) -> Result<&Self, ProjectGraphError> {
        // TODO: Don't clone data here, but satisfying the borrow checker
        // is almost impossible here without a major refactor!
        let ids = self
            .sources
            .keys()
            .map(|k| k.to_owned())
            .collect::<Vec<String>>();

        for id in ids {
            self.internal_load(&id)?;
        }

        Ok(self)
    }

    /// Create a project with the provided ID and file path source. Based on the project's
    /// configured language, detect and infer implicit dependencies and tasks for the
    /// matching platform. Do *not* expand tasks until after dependents have been created.
    fn create_project(&self, id: &str, source: &str) -> Result<Project, ProjectGraphError> {
        let mut project = Project::new(id, source, self.workspace_root, self.config)?;

        // Collect all aliases for the current project ID
        for (alias, project_id) in &self.aliases {
            if project_id == id {
                project.aliases.push(alias.to_owned());
            }
        }

        // Detect the language if its unknown
        if matches!(project.language, ProjectLanguage::Unknown) {
            project.language = self.platforms.detect_project_language(&project.root);
        }

        if let Some(platform) = self.platforms.get(project.language) {
            // Inherit implicit dependencies
            for dep_config in platform.load_project_implicit_dependencies(
                id,
                &project.root,
                &project.config,
                &self.aliases,
            )? {
                // Implicit must not override explicit
                project
                    .dependencies
                    .entry(dep_config.id.clone())
                    .or_insert_with(|| {
                        let mut dep = ProjectDependency::from_config(&dep_config);
                        dep.source = ProjectDependencySource::Implicit;
                        dep
                    });
            }

            // Inherit platform specific tasks
            for (task_id, task_config) in
                platform.load_project_tasks(id, &project.root, &project.config)?
            {
                // Inferred mut not override explicit
                #[allow(clippy::map_entry)]
                if !project.tasks.contains_key(&task_id) {
                    let task = Task::from_config(Target::new(id, &task_id)?, &task_config)?;

                    project.tasks.insert(task_id, task);
                }
            }
        }

        Ok(project)
    }

    /// Expand all tasks within a project, by expanding data and resolving any tokens.
    /// This must run *after* dependent projects have been created, as we require them
    /// to resolve "parent" relations.
    fn expand_project(&mut self, project: &mut Project) -> Result<(), ProjectGraphError> {
        let mut tasks = BTreeMap::new();

        // Use `mem::take` so that we can mutably borrow the project and tasks in parallel
        for (task_id, mut task) in mem::take(&mut project.tasks) {
            if matches!(task.platform, PlatformType::Unknown) {
                task.platform = TaskConfig::detect_platform(&project.config, &task.command);
            }

            // Resolve in this order!
            self.expand_task_env(project, &mut task)?;
            self.expand_task_deps(project, &mut task)?;
            self.expand_task_inputs(project, &mut task)?;
            self.expand_task_outputs(project, &mut task)?;
            self.expand_task_args(project, &mut task)?;

            // Determine type after expanding
            task.determine_type();

            tasks.insert(task_id, task);
        }

        project.tasks.extend(tasks);

        Ok(())
    }

    /// Expand the args list to resolve tokens, relative to the project root.
    pub fn expand_task_args(
        &self,
        project: &mut Project,
        task: &mut Task,
    ) -> Result<(), ProjectGraphError> {
        if task.args.is_empty() {
            return Ok(());
        }

        let mut args: Vec<String> = vec![];

        // When running within a project:
        //  - Project paths are relative and start with "./"
        //  - Workspace paths are relative up to the root
        // When running from the workspace:
        //  - All paths are absolute
        let handle_path = |path: PathBuf, is_glob: bool| -> Result<String, ProjectGraphError> {
            let arg = if !task.options.run_from_workspace_root
                && path.starts_with(self.workspace_root)
            {
                let rel_path = path::to_string(path::relative_from(&path, &project.root).unwrap())?;

                if rel_path.starts_with("..") {
                    rel_path
                } else {
                    format!(".{}{}", std::path::MAIN_SEPARATOR, rel_path)
                }
            } else {
                path::to_string(path)?
            };

            // Annoying, but we need to force forward slashes,
            // and remove drive/UNC prefixes...
            if cfg!(windows) && is_glob {
                return Ok(glob::remove_drive_prefix(path::standardize_separators(arg)));
            }

            Ok(arg)
        };

        // We cant use `TokenResolver.resolve` as args are a mix of strings,
        // strings with tokens, and file paths when tokens are resolved.
        let token_resolver = TokenResolver::new(TokenContext::Args, project, self.workspace_root);

        for arg in &task.args {
            if token_resolver.has_token_func(arg) {
                let (paths, globs) = token_resolver.resolve_func(arg, task)?;

                for path in paths {
                    args.push(handle_path(path, false)?);
                }

                for glob in globs {
                    args.push(handle_path(PathBuf::from(glob), true)?);
                }
            } else if token_resolver.has_token_var(arg) {
                args.push(token_resolver.resolve_vars(arg, task)?);
            } else {
                args.push(arg.clone());
            }
        }

        task.args = args;

        Ok(())
    }

    /// Expand the deps list and resolve parent/self scopes.
    pub fn expand_task_deps(
        &self,
        project: &mut Project,
        task: &mut Task,
    ) -> Result<(), ProjectGraphError> {
        if !self.workspace_config.runner.implicit_deps.is_empty() {
            task.deps.extend(Task::create_dep_targets(
                &self.workspace_config.runner.implicit_deps,
            )?);
        }

        if task.deps.is_empty() {
            return Ok(());
        }

        let mut dep_targets: Vec<Target> = vec![];

        // Dont use a `HashSet` as we want to preserve order
        let mut push_target = |dep: Target| {
            if !dep_targets.contains(&dep) {
                dep_targets.push(dep);
            }
        };

        for target in &task.deps {
            match &target.project {
                // ^:task
                TargetProjectScope::Deps => {
                    for dep_id in project.get_dependency_ids() {
                        let dep_index = self.indices.get(dep_id).unwrap();
                        let dep_project = self.graph.node_weight(*dep_index).unwrap();

                        if let Some(dep_task) = dep_project.tasks.get(&target.task_id) {
                            push_target(dep_task.target.clone());
                        }
                    }
                }
                // ~:task
                TargetProjectScope::OwnSelf => {
                    if target.task_id != task.id {
                        push_target(Target::new(&project.id, &target.task_id)?);
                    }
                }
                // project:task
                TargetProjectScope::Id(project_id) => {
                    if project_id == &project.id && target.task_id == task.id {
                        // Avoid circular references
                    } else {
                        push_target(target.clone());
                    }
                }
                _ => {
                    target.fail_with(TargetError::NoProjectAllInTaskDeps(target.id.clone()))?;
                }
            };
        }

        task.deps = dep_targets;

        Ok(())
    }

    /// Expand environment variables by loading a `.env` file if configured.
    pub fn expand_task_env(
        &self,
        project: &mut Project,
        task: &mut Task,
    ) -> Result<(), ProjectGraphError> {
        let Some(env_file) = &task.options.env_file else {
            return Ok(());
        };

        let env_path = project.root.join(env_file);
        let error_handler =
            |e: dotenvy::Error| TaskError::InvalidEnvFile(env_path.clone(), e.to_string());

        // Add as an input
        task.inputs.push(env_file.to_owned());

        // The `.env` file may not have been committed, so avoid crashing in CI
        if is_ci() && !env_path.exists() {
            debug!(
                target: task.get_log_target(),
                "The `envFile` option is enabled but no `.env` file exists in CI, skipping as this may be intentional",
            );

            return Ok(());
        }

        for entry in dotenvy::from_path_iter(&env_path).map_err(error_handler)? {
            let (key, value) = entry.map_err(error_handler)?;

            // Vars defined in `env` take precedence over those in the env file
            task.env.entry(key).or_insert(value);
        }

        Ok(())
    }

    /// Expand the inputs list to a set of absolute file paths, while resolving tokens.
    pub fn expand_task_inputs(
        &self,
        project: &mut Project,
        task: &mut Task,
    ) -> Result<(), ProjectGraphError> {
        if !self.workspace_config.runner.implicit_inputs.is_empty() {
            task.inputs
                .extend(self.workspace_config.runner.implicit_inputs.clone());
        }

        if task.inputs.is_empty() {
            return Ok(());
        }

        let inputs_without_vars = task
            .inputs
            .iter()
            .filter(|input| {
                if ENV_VAR.is_match(input) {
                    task.input_vars.insert(input[1..].to_owned());
                    false
                } else {
                    true
                }
            })
            .map(|input| input.to_owned())
            .collect::<Vec<_>>();

        let token_resolver = TokenResolver::new(TokenContext::Inputs, project, self.workspace_root);
        let (paths, globs) = token_resolver.resolve(&inputs_without_vars, task)?;

        task.input_paths.extend(paths);
        task.input_globs.extend(globs);

        Ok(())
    }

    /// Expand the outputs list to a set of absolute file paths, while resolving tokens.
    pub fn expand_task_outputs(
        &self,
        project: &mut Project,
        task: &mut Task,
    ) -> Result<(), ProjectGraphError> {
        if task.outputs.is_empty() {
            return Ok(());
        }

        let token_resolver =
            TokenResolver::new(TokenContext::Outputs, project, self.workspace_root);
        let (paths, globs) = token_resolver.resolve(&task.outputs, task)?;

        task.output_paths.extend(paths);

        if !globs.is_empty() {
            return Err(ProjectGraphError::Task(TaskError::NoOutputGlob(
                globs.get(0).unwrap().to_owned(),
                task.target.id.clone(),
            )));
        }

        Ok(())
    }

    fn internal_load(&mut self, alias_or_id: &str) -> Result<NodeIndex, ProjectGraphError> {
        let id = match self.aliases.get(alias_or_id) {
            Some(project_id) => project_id,
            None => alias_or_id,
        };

        // Already loaded, abort early
        if let Some(index) = self.indices.get(id) {
            trace!(
                target: LOG_TARGET,
                "Project {} already exists in the project graph",
                color::id(id),
            );

            return Ok(*index);
        }

        trace!(
            target: LOG_TARGET,
            "Project {} does not exist in the project graph, attempting to load",
            color::id(id),
        );

        // Create the current project
        let id = id.to_owned();
        let Some(source) = self.sources.get(&id) else {
            return Err(ProjectGraphError::Project(ProjectError::UnconfiguredID(id)));
        };

        let mut project = self.create_project(&id, source)?;

        // Create dependent projects
        let mut dep_indices = FxHashSet::default();

        for dep_id in project.get_dependency_ids() {
            dep_indices.insert(self.internal_load(dep_id)?);
        }

        // Expand tasks for the current project
        self.expand_project(&mut project)?;

        // Insert into the graph and connect edges
        let index = self.graph.add_node(project);

        self.indices.insert(id, index);

        for dep_index in dep_indices {
            self.graph.add_edge(index, dep_index, ());
        }

        Ok(index)
    }

    fn load_aliases(&mut self) -> Result<(), ProjectGraphError> {
        for platform in self.platforms.list_mut() {
            platform.load_project_graph_aliases(&self.sources, &mut self.aliases)?;
        }

        Ok(())
    }

    fn load_sources(&mut self) -> Result<(), ProjectGraphError> {
        let mut globs = vec![];
        let mut sources = FxHashMap::default();

        match &self.workspace_config.projects {
            WorkspaceProjects::Sources(map) => {
                sources.extend(map.clone());
            }
            WorkspaceProjects::Globs(list) => {
                globs.extend(list.clone());
            }
            WorkspaceProjects::Both {
                globs: list,
                sources: map,
            } => {
                globs.extend(list.clone());
                sources.extend(map.clone());
            }
        };

        // Only check the cache when using globs
        if !globs.is_empty() {
            let mut cache = self.cache.cache_projects_state()?;

            // Return the values from the cache
            if !cache.projects.is_empty() {
                debug!(target: LOG_TARGET, "Loading projects from cache");

                self.sources.extend(cache.projects);

                return Ok(());
            }

            // Generate a new projects map by globbing the filesystem
            debug!(
                target: LOG_TARGET,
                "Finding projects with globs: {}",
                map_list(&globs, |g| color::file(g))
            );

            detect_projects_with_globs(self.workspace_root, &globs, &mut sources)?;

            // Update the cache
            cache.globs = globs.clone();
            cache.projects = sources.clone();
            cache.save()?;
        }

        debug!(
            target: LOG_TARGET,
            "Creating project graph with {} projects",
            sources.len(),
        );

        self.sources.extend(sources);

        Ok(())
    }
}
