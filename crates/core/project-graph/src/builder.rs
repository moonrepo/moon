use crate::project_graph::{GraphType, IndicesType, ProjectGraph, LOG_TARGET, ROOT_NODE_ID};
use moon_cache::CacheEngine;
use moon_config::{
    GlobalProjectConfig, ProjectsAliasesMap, ProjectsSourcesMap, ToolchainConfig, WorkspaceConfig,
    WorkspaceProjects,
};
use moon_logger::{color, debug, map_list, trace};
use moon_platform::PlatformManager;
use moon_project::{
    detect_projects_with_globs, Project, ProjectDependency, ProjectDependencySource, ProjectError,
};
use moon_task::{Target, Task};
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use std::path::Path;

pub struct ProjectGraphBuilder<'graph> {
    pub cache: &'graph CacheEngine,
    pub config: &'graph GlobalProjectConfig,
    pub platforms: &'graph mut PlatformManager,
    pub toolchain_config: &'graph ToolchainConfig,
    pub workspace_config: &'graph WorkspaceConfig,
    pub workspace_root: &'graph Path,
}

impl<'graph> ProjectGraphBuilder<'graph> {
    pub async fn build(&mut self) -> Result<ProjectGraph, ProjectError> {
        let sources = self.load_sources().await?;
        let aliases = self.load_aliases(&sources).await?;
        let (graph, indices) = self.build_graph(&sources, &aliases)?;

        Ok(ProjectGraph::new(graph, indices, sources, aliases))
    }

    fn build_graph(
        &mut self,
        sources: &ProjectsSourcesMap,
        aliases: &ProjectsAliasesMap,
    ) -> Result<(GraphType, IndicesType), ProjectError> {
        let mut graph = DiGraph::new();
        let mut indices = FxHashMap::default();

        // Add a virtual root node
        graph.add_node(Project {
            id: ROOT_NODE_ID.to_owned(),
            root: self.workspace_root.to_path_buf(),
            source: String::from("."),
            ..Project::default()
        });

        // Add a node for each project
        for id in sources.keys() {
            self.load(&mut graph, &mut indices, sources, aliases, &id)?;
        }

        Ok((graph, indices))
    }

    fn create_project(
        &self,
        aliases: &ProjectsAliasesMap,
        id: &str,
        source: &str,
    ) -> Result<Project, ProjectError> {
        let mut project = Project::new(id, source, &self.workspace_root, &self.config)?;

        // Find the alias for a given ID. This is currently... not performant,
        // so revisit once it becomes an issue!
        for (alias, project_id) in aliases {
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
                aliases,
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
            for (task_id, task_config) in platform.load_project_tasks(
                id,
                &project.root,
                &project.config,
                &self.workspace_root,
                &self.toolchain_config,
            )? {
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
            &self.workspace_root,
            &self.workspace_config.runner.implicit_deps,
            &self.workspace_config.runner.implicit_inputs,
        )?;

        Ok(project)
    }

    fn load(
        &mut self,
        graph: &mut GraphType,
        indices: &mut IndicesType,
        sources: &ProjectsSourcesMap,
        aliases: &ProjectsAliasesMap,
        alias_or_id: &str,
    ) -> Result<NodeIndex, ProjectError> {
        let id = match aliases.get(alias_or_id) {
            Some(project_id) => project_id,
            None => alias_or_id,
        };

        // Already loaded, abort early
        if indices.contains_key(id) || id == ROOT_NODE_ID {
            trace!(
                target: LOG_TARGET,
                "Project {} already exists in the project graph",
                color::id(&id),
            );

            return Ok(*indices.get(id).unwrap());
        }

        trace!(
            target: LOG_TARGET,
            "Project {} does not exist in the project graph, attempting to load",
            color::id(&id),
        );

        // Create project based on ID and source
        let Some(source) = sources.get(id) else {
            return Err(ProjectError::UnconfiguredID(id.to_owned()));
        };

        let project = self.create_project(aliases, &id, source)?;
        let depends_on = project.get_dependency_ids();

        // Insert the project into the graph
        let node_index = graph.add_node(project);

        graph.add_edge(NodeIndex::new(0), node_index, ());
        indices.insert(id.to_owned(), node_index);

        if !depends_on.is_empty() {
            trace!(
                target: LOG_TARGET,
                "Adding dependencies {} to project {}",
                map_list(&depends_on, |d| color::symbol(d)),
                color::id(id),
            );

            for dep_id in depends_on {
                let dep_index = self.load(graph, indices, sources, aliases, dep_id.as_str())?;

                graph.add_edge(node_index, dep_index, ());
            }
        }

        Ok(node_index)
    }

    async fn load_aliases(
        &mut self,
        sources: &ProjectsSourcesMap,
    ) -> Result<ProjectsAliasesMap, ProjectError> {
        let mut aliases = FxHashMap::default();

        for platform in self.platforms.list_mut() {
            platform.load_project_graph_aliases(
                &self.workspace_root,
                &self.toolchain_config,
                sources,
                &mut aliases,
            )?;
        }

        Ok(aliases)
    }

    async fn load_sources(&mut self) -> Result<ProjectsSourcesMap, ProjectError> {
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

                return Ok(cache.projects);
            }

            // Generate a new projects map by globbing the filesystem
            debug!(
                target: LOG_TARGET,
                "Finding projects with globs: {}",
                map_list(&globs, |g| color::file(g))
            );

            detect_projects_with_globs(&self.workspace_root, &globs, &mut sources)?;

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

        Ok(sources)
    }
}
