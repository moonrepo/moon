use crate::project_graph::{GraphType, IndicesType, ProjectGraph, LOG_TARGET};
use moon_cache::CacheEngine;
use moon_config::{
    GlobalProjectConfig, ProjectsAliasesMap, ProjectsSourcesMap, WorkspaceConfig, WorkspaceProjects,
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
    pub async fn new(
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

        graph.load_sources().await?;
        graph.load_aliases().await?;

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

        // Expand all tasks for the project (this must happen last)
        project.expand_tasks(
            self.workspace_root,
            &self.workspace_config.runner.implicit_deps,
            &self.workspace_config.runner.implicit_inputs,
        )?;

        Ok(project)
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
        let depends_on = project.get_dependency_ids();

        // Insert the project into the graph
        let node_index = self.graph.add_node(project);

        self.indices.insert(id.to_owned(), node_index);

        if !depends_on.is_empty() {
            trace!(
                target: LOG_TARGET,
                "Adding dependencies {} to project {}",
                map_list(&depends_on, |d| color::symbol(d)),
                color::id(id),
            );

            for dep_id in depends_on {
                let dep_index = self.internal_load(dep_id.as_str())?;

                self.graph.add_edge(node_index, dep_index, ());
            }
        }

        Ok(node_index)
    }

    async fn load_aliases(&mut self) -> Result<(), ProjectError> {
        for platform in self.platforms.list_mut() {
            platform.load_project_graph_aliases(&self.sources, &mut self.aliases)?;
        }

        Ok(())
    }

    async fn load_sources(&mut self) -> Result<(), ProjectError> {
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
            let mut cache = self.cache.cache_projects_state().await?;

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
            cache.save().await?;
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
