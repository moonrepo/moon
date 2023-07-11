use crate::project_graph::GraphType;
use async_recursion::async_recursion;
use moon_common::path::{WorkspaceRelativePath, WorkspaceRelativePathBuf};
use moon_common::Id;
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig, WorkspaceProjects};
use moon_project::Project;
use moon_project_builder::{
    DetectLanguageEvent, ProjectBuilder, ProjectBuilderContext, ProjectBuilderError,
};
use moon_task_builder::DetectPlatformEvent;
use petgraph::graph::DiGraph;
use petgraph::prelude::NodeIndex;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_events::Emitter;
use std::path::Path;
use tracing::{trace, warn};

pub struct ProjectGraphBuilderContext<'app> {
    pub detect_language: &'app Emitter<DetectLanguageEvent>,
    pub detect_platform: &'app Emitter<DetectPlatformEvent>,
    pub inherited_tasks: &'app InheritedTasksManager,
    pub toolchain_config: &'app ToolchainConfig,
    pub workspace_config: &'app WorkspaceConfig,
    pub workspace_root: &'app Path,
}

pub struct ProjectNode {
    index: NodeIndex,
}

pub struct ProjectGraphBuilder<'app> {
    context: ProjectGraphBuilderContext<'app>,

    /// The DAG instance.
    graph: GraphType,

    /// Nodes (projects) inserted into the graph.
    nodes: FxHashMap<Id, ProjectNode>,

    /// Mapping of project IDs to file system sources,
    /// derived from the `workspace.projects` setting.
    sources: FxHashMap<Id, WorkspaceRelativePathBuf>,
}

impl<'app> ProjectGraphBuilder<'app> {
    pub fn new(context: ProjectGraphBuilderContext<'app>) -> miette::Result<Self> {
        let mut graph = ProjectGraphBuilder {
            context,
            graph: DiGraph::new(),
            nodes: FxHashMap::default(),
            sources: FxHashMap::default(),
        };

        graph.preload()?;

        Ok(graph)
    }

    pub async fn load(&mut self, alias_or_id: &str) -> miette::Result<()> {
        self.internal_load(alias_or_id, &mut FxHashSet::default())
            .await?;

        Ok(())
    }

    pub async fn load_all(&mut self) -> miette::Result<()> {
        Ok(())
    }

    #[async_recursion]
    async fn internal_load(
        &mut self,
        alias_or_id: &str,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<NodeIndex> {
        let id = self.resolve_id(alias_or_id);

        // Already loaded, exit early with existing index
        if let Some(node) = self.nodes.get(&id) {
            trace!(
                project_id = id.as_str(),
                "Project already exists in the project graph",
            );

            return Ok(node.index);
        }

        // Check that the project ID is configured
        trace!(
            project_id = id.as_str(),
            "Project does not exist in the project graph, attempting to load",
        );

        let Some(source) = self.sources.get(&id) else {
            return Err(ProjectBuilderError::UnconfiguredID(id).into());
        };

        // Create the project
        let project = self.create_project(&id, source).await?;

        cycle.insert(id.clone());

        // Create dependent projects
        let mut edges = vec![];

        for (dep_id, dep_config) in &project.dependencies {
            if cycle.contains(dep_id) {
                warn!(
                    project_id = id.as_str(),
                    dependency_id = dep_id.as_str(),
                    "Encountered a dependency cycle; will disconnect nodes to avoid recursion",
                );
            } else {
                edges.push((self.internal_load(dep_id, cycle).await?, dep_config.scope));
            }
        }

        // Insert into the graph and connect edges
        let index = self.graph.add_node(project);

        for edge in edges {
            self.graph.add_edge(index, edge.0, edge.1);
        }

        self.nodes.insert(id, ProjectNode { index });

        Ok(index)
    }

    async fn create_project(
        &self,
        id: &Id,
        source: &WorkspaceRelativePath,
    ) -> miette::Result<Project> {
        let mut builder = ProjectBuilder::new(
            id,
            source.as_str(), // TODO?
            ProjectBuilderContext {
                detect_language: self.context.detect_language,
                detect_platform: self.context.detect_platform,
                toolchain_config: self.context.toolchain_config,
                workspace_root: self.context.workspace_root,
            },
        )?;

        builder.load_local_config().await?;
        builder.inherit_global_config(self.context.inherited_tasks)?;

        // if let Ok(platform) = self.workspace.platforms.get(builder.language.clone()) {
        //     // Inherit implicit dependencies
        //     for dep_config in
        //         platform.load_project_implicit_dependencies(id, source, &self.aliases)?
        //     {
        //         builder.extend_with_dependency(dep_config);
        //     }

        //     // Inherit platform specific tasks
        //     for (task_id, task_config) in platform.load_project_tasks(id, source)? {
        //         builder.extend_with_task(task_id, task_config);
        //     }
        // }

        let project = builder.build().await?;

        // TODO
        // for (alias, project_id) in &self.aliases {
        //     if project_id == id {
        //         project.alias = Some(alias.to_owned());
        //     }
        // }

        Ok(project)
    }

    fn preload(&mut self) -> miette::Result<()> {
        let mut globs = vec![];
        let mut sources = FxHashMap::default();

        let mut add_sources = |map: &FxHashMap<Id, String>| {
            for (id, source) in map {
                sources.insert(id.to_owned(), WorkspaceRelativePathBuf::from(source));
            }
        };

        match &self.context.workspace_config.projects {
            WorkspaceProjects::Sources(map) => {
                add_sources(map);
            }
            WorkspaceProjects::Globs(list) => {
                globs.extend(list.clone());
            }
            WorkspaceProjects::Both(cfg) => {
                globs.extend(cfg.globs.clone());
                add_sources(&cfg.sources);
            }
        };

        // TODO glob
        // TODO load aliases

        self.sources = sources;

        Ok(())
    }

    fn resolve_id(&self, alias_or_id: &str) -> Id {
        // Id::raw(match self.aliases.get(alias_or_id) {
        //     Some(project_id) => project_id,
        //     None => alias_or_id,
        // })
        Id::raw(alias_or_id)
    }
}
