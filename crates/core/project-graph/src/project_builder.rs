use crate::errors::ProjectGraphError;
use crate::graph_hasher::GraphHasher;
use crate::helpers::detect_projects_with_globs;
use crate::project_graph::{GraphType, IndicesType, ProjectGraph, LOG_TARGET};
use crate::token_resolver::{TokenContext, TokenResolver};
use moon_config::{
    ProjectsAliasesMap, ProjectsSourcesMap, WorkspaceProjects, CONFIG_DIRNAME,
    CONFIG_PROJECT_FILENAME,
};
use moon_error::MoonError;
use moon_hasher::{convert_paths_to_strings, to_hash};
use moon_logger::{color, debug, map_list, trace, Logable};
use moon_platform_detector::{detect_project_language, detect_task_platform};
use moon_project::{Project, ProjectDependency, ProjectDependencySource, ProjectError};
use moon_target::{Target, TargetError, TargetProjectScope};
use moon_task::{Task, TaskError};
use moon_utils::regex::ENV_VAR;
use moon_utils::{glob, is_ci, path, time};
use moon_workspace::Workspace;
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::mem;
use std::path::PathBuf;
use std::time::Duration;

pub struct ProjectGraphBuilder<'ws> {
    workspace: &'ws mut Workspace,

    aliases: ProjectsAliasesMap,
    graph: GraphType,
    indices: IndicesType,
    sources: ProjectsSourcesMap,

    pub is_cached: bool,
    pub hash: String,
}

impl<'ws> ProjectGraphBuilder<'ws> {
    pub async fn new(
        workspace: &'ws mut Workspace,
    ) -> Result<ProjectGraphBuilder<'ws>, ProjectGraphError> {
        debug!(target: LOG_TARGET, "Creating project graph");

        let mut graph = ProjectGraphBuilder {
            aliases: FxHashMap::default(),
            graph: DiGraph::new(),
            hash: String::new(),
            indices: FxHashMap::default(),
            is_cached: false,
            sources: FxHashMap::default(),
            workspace,
        };

        graph.preload().await?;

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
        let mut project = Project::new(
            id,
            source,
            &self.workspace.root,
            &self.workspace.tasks_config,
            detect_project_language,
        )?;

        // Collect all aliases for the current project ID
        for (alias, project_id) in &self.aliases {
            if project_id == id {
                project.aliases.push(alias.to_owned());
            }
        }

        if let Ok(platform) = self.workspace.platforms.get(project.language) {
            // Inherit implicit dependencies
            for dep_config in
                platform.load_project_implicit_dependencies(&project, &self.aliases)?
            {
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
            for (task_id, task_config) in platform.load_project_tasks(&project)? {
                // Inferred must not override explicit
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
        let project_platform = project.config.platform.unwrap_or_default();

        // Use `mem::take` so that we can mutably borrow the project and tasks in parallel
        for (task_id, mut task) in mem::take(&mut project.tasks) {
            // Detect the platform if its unknown
            if task.platform.is_unknown() {
                task.platform = if project_platform.is_unknown() {
                    detect_task_platform(&task.command, project.language)
                } else {
                    project_platform
                };
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
                && path.starts_with(&self.workspace.root)
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
        let token_resolver = TokenResolver::new(TokenContext::Args, project, &self.workspace.root);

        for arg in &task.args {
            if token_resolver.has_token_func(arg) {
                let (paths, globs) = token_resolver.resolve_func(arg, task)?;

                for path in paths {
                    args.push(handle_path(path, false)?);
                }

                for glob in globs {
                    args.push(handle_path(glob, true)?);
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
        if !project.inherited_config.implicit_deps.is_empty() {
            task.deps.extend(Task::create_dep_targets(
                &project.inherited_config.implicit_deps,
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
        // Load from env file
        if let Some(env_file) = &task.options.env_file {
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
            } else {
                for entry in dotenvy::from_path_iter(&env_path).map_err(error_handler)? {
                    let (key, value) = entry.map_err(error_handler)?;

                    // Vars defined in task `env` take precedence over those in the env file
                    task.env.entry(key).or_insert(value);
                }
            }
        }

        // Inherit project-level
        if let Some(project_env) = &project.config.env {
            for (key, value) in project_env {
                // Vars defined in task `env` take precedence
                task.env
                    .entry(key.to_owned())
                    .or_insert_with(|| value.to_owned());
            }
        }

        Ok(())
    }

    /// Expand the inputs list to a set of absolute file paths, while resolving tokens.
    pub fn expand_task_inputs(
        &self,
        project: &mut Project,
        task: &mut Task,
    ) -> Result<(), ProjectGraphError> {
        if !project.inherited_config.implicit_inputs.is_empty() {
            task.inputs
                .extend(project.inherited_config.implicit_inputs.clone());
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

        let token_resolver =
            TokenResolver::new(TokenContext::Inputs, project, &self.workspace.root);
        let (paths, globs) = token_resolver.resolve(&inputs_without_vars, task)?;

        task.input_paths.extend(paths);
        task.input_globs.extend(self.normalize_glob_list(globs)?);

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
            TokenResolver::new(TokenContext::Outputs, project, &self.workspace.root);
        let (paths, globs) = token_resolver.resolve(&task.outputs, task)?;

        task.output_paths.extend(paths);
        task.output_globs.extend(self.normalize_glob_list(globs)?);

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

    async fn preload(&mut self) -> Result<(), ProjectGraphError> {
        let mut globs = vec![];
        let mut sources = FxHashMap::default();
        let mut aliases = FxHashMap::default();
        let mut cache = self.workspace.cache.cache_projects_state()?;

        // Load project sources
        match &self.workspace.config.projects {
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

        if !globs.is_empty() {
            if time::is_stale(cache.last_glob_time, Duration::from_secs(60 * 5)) {
                debug!(
                    target: LOG_TARGET,
                    "Finding projects with globs: {}",
                    map_list(&globs, |g| color::file(g))
                );

                detect_projects_with_globs(&self.workspace.root, &globs, &mut sources)?;

                cache.last_glob_time = time::now_millis();
            } else {
                sources.extend(cache.projects);
            }
        }

        // Load project aliases
        for platform in self.workspace.platforms.list_mut() {
            platform.load_project_graph_aliases(&sources, &mut aliases)?;
        }

        // Update the cache
        let hash = self.generate_hash(&sources, &aliases).await?;

        if !hash.is_empty() {
            self.is_cached = cache.last_hash == hash;
            self.hash = hash.clone();

            debug!(
                target: LOG_TARGET,
                "Generated hash {} for project graph",
                color::hash(&hash),
            );
        }

        self.aliases.extend(aliases.clone());
        self.sources.extend(sources.clone());

        cache.last_hash = hash;
        cache.globs = globs;
        cache.projects = sources;
        cache.save()?;

        if self.is_cached {
            debug!(
                target: LOG_TARGET,
                "Loading project graph with {} projects from cache",
                self.sources.len(),
            );
        } else {
            debug!(
                target: LOG_TARGET,
                "Creating project graph with {} projects",
                self.sources.len(),
            );
        }

        Ok(())
    }

    async fn generate_hash(
        &self,
        sources: &ProjectsSourcesMap,
        aliases: &ProjectsAliasesMap,
    ) -> Result<String, MoonError> {
        if !self.workspace.vcs.is_enabled() {
            return Ok(String::new());
        }

        let mut hasher = GraphHasher::default();

        // Hash aliases and sources as-is as they're very explicit
        hasher.hash_aliases(aliases);
        hasher.hash_sources(sources);

        // Hash all project-oriented config files, as a single change in any of
        // these files would invalidate the entire project graph cache!
        // TODO: handle extended config files?
        let configs = convert_paths_to_strings(
            &FxHashSet::from_iter(
                sources
                    .values()
                    .map(|source| PathBuf::from(source).join(CONFIG_PROJECT_FILENAME)),
            ),
            &self.workspace.root,
        )?;

        let config_hashes = self
            .workspace
            .vcs
            .get_file_hashes(&configs, false)
            .await
            .map_err(|e| MoonError::Generic(e.to_string()))?;

        hasher.hash_configs(&config_hashes);

        let config_hashes = self
            .workspace
            .vcs
            .get_file_tree_hashes(CONFIG_DIRNAME)
            .await
            .map_err(|e| MoonError::Generic(e.to_string()))?;

        hasher.hash_configs(&config_hashes);

        // Generate the hash
        let hash = to_hash(&hasher);

        self.workspace.cache.create_hash_manifest(&hash, &hasher)?;

        Ok(hash)
    }

    fn normalize_glob_list(&self, globs: Vec<PathBuf>) -> Result<Vec<String>, ProjectError> {
        let mut normalized_globs = vec![];

        for glob in globs {
            normalized_globs.push(glob::normalize(
                //  glob.strip_prefix(&self.workspace.root).unwrap(),
                glob,
            )?);
        }

        Ok(normalized_globs)
    }
}
