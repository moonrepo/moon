use crate::project_graph::{GraphType, IndicesType, ProjectGraph, LOG_TARGET};
use crate::task_expander::TaskExpander;
use moon_cache::CacheEngine;
use moon_config::{
    GlobalProjectConfig, PlatformType, ProjectsAliasesMap, ProjectsSourcesMap, TaskConfig,
    WorkspaceConfig, WorkspaceProjects,
};
use moon_logger::{color, debug, map_list, trace};
use moon_platform::PlatformManager;
use moon_project::{
    detect_projects_with_globs, Project, ProjectDependency, ProjectDependencySource, ProjectError,
};
use moon_task::{Target, Task};
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use std::mem;
use std::path::Path;

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
    ) -> Result<ProjectGraphBuilder<'ws>, ProjectError> {
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

    pub fn load(&mut self, alias_or_id: &str) -> Result<&Self, ProjectError> {
        self.internal_load(alias_or_id)?;

        Ok(self)
    }

    pub fn load_all(&mut self) -> Result<&Self, ProjectError> {
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

    fn create_project(&self, id: &str, source: &str) -> Result<Project, ProjectError> {
        let mut project = Project::new(id, source, self.workspace_root, self.config)?;

        // Find the alias for a given ID. This is currently... not performant,
        // so revisit once it becomes an issue!
        for (alias, project_id) in &self.aliases {
            if project_id == id {
                project.alias = Some(alias.to_owned());
                break;
            }
        }

        for platform in self.platforms.list() {
            if !platform.matches(&project.config.language.to_platform(), None) {
                continue;
            }

            // Determine implicit dependencies
            for dep_cfg in platform.load_project_implicit_dependencies(
                id,
                &project.root,
                &project.config,
                &self.aliases,
            )? {
                // Implicit deps should not override explicit deps
                project
                    .dependencies
                    .entry(dep_cfg.id.clone())
                    .or_insert_with(|| {
                        let mut dep = ProjectDependency::from_config(&dep_cfg);
                        dep.source = ProjectDependencySource::Implicit;
                        dep
                    });
            }

            // Inherit platform specific tasks
            for (task_id, task_config) in
                platform.load_project_tasks(id, &project.root, &project.config)?
            {
                // Inferred tasks should not override explicit tasks
                #[allow(clippy::map_entry)]
                if !project.tasks.contains_key(&task_id) {
                    let task = Task::from_config(Target::new(id, &task_id)?, &task_config)?;

                    project.tasks.insert(task_id, task);
                }
            }
        }

        Ok(project)
    }

    fn expand_tasks(&mut self, index: &NodeIndex) -> Result<(), ProjectError> {
        let project = self.graph.node_weight_mut(*index).unwrap();
        let mut dep_projects = FxHashMap::default();

        // Find all dependent projects
        for dep_id in project.dependencies.keys() {
            // dep_projects.insert(dep_id.to_owned(), self.load(&dep_id).unwrap());
        }

        // Expand all tasks and resolve tokens
        let task_expander = TaskExpander::new(&project, &self.workspace_root);

        for task in project.tasks.values_mut() {
            // Inherit implicits before resolving
            task.deps.extend(Task::create_dep_targets(
                &self.workspace_config.runner.implicit_deps,
            )?);

            task.inputs
                .extend(self.workspace_config.runner.implicit_inputs.iter().cloned());

            // Resolve in this order!
            task_expander.expand_env(task)?;
            task_expander.expand_deps(task, &project.id, &dep_projects)?;
            task_expander.expand_inputs(task)?;
            task_expander.expand_outputs(task)?;
            task_expander.expand_args(task)?;

            if matches!(task.platform, PlatformType::Unknown) {
                task.platform = TaskConfig::detect_platform(&project.config, &task.command);
            }
        }

        Ok(())
    }

    fn internal_load(&mut self, alias_or_id: &str) -> Result<NodeIndex, ProjectError> {
        let id = match self.aliases.get(alias_or_id) {
            Some(project_id) => project_id,
            None => alias_or_id,
        };

        // Already loaded, abort early
        if self.indices.contains_key(id) {
            trace!(
                target: LOG_TARGET,
                "Project {} already exists in the project graph",
                color::id(id),
            );

            return Ok(*self.indices.get(id).unwrap());
        }

        trace!(
            target: LOG_TARGET,
            "Project {} does not exist in the project graph, attempting to load",
            color::id(id),
        );

        // Create project based on ID and source
        let Some(source) = self.sources.get(id) else {
            return Err(ProjectError::UnconfiguredID(id.to_owned()));
        };

        let project = self.create_project(id, source)?;

        // Insert the project into the graph
        let node_index = self.graph.add_node(project);

        self.indices.insert(id.to_owned(), node_index);

        // Create dependent projects
        let depends_on = project.dependencies.keys();

        if depends_on.len() > 0 {
            // trace!(
            //     target: LOG_TARGET,
            //     "Adding dependencies {} to project {}",
            //     map_list(&depends_on, |d| color::symbol(d)),
            //     color::id(id),
            // );

            for dep_id in depends_on {
                let dep_index = self.internal_load(dep_id)?;

                self.graph.add_edge(node_index, dep_index, ());
            }
        }

        // Expand tasks for the new project
        self.expand_tasks(&node_index)?;

        Ok(node_index)
    }

    fn load_aliases(&mut self) -> Result<(), ProjectError> {
        for platform in self.platforms.list_mut() {
            platform.load_project_graph_aliases(&self.sources, &mut self.aliases)?;
        }

        Ok(())
    }

    fn load_sources(&mut self) -> Result<(), ProjectError> {
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
