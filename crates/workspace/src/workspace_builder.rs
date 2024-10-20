use crate::projects_locator::locate_projects_with_globs;
use crate::repo_type::RepoType;
use crate::workspace_builder_error::WorkspaceBuilderError;
use moon_common::path::is_root_level_source;
use moon_common::{color, path::WorkspaceRelativePathBuf, Id};
use moon_config::{
    ConfigLoader, DependencyScope, InheritedTasksManager, ProjectConfig, ProjectsSourcesList,
    ToolchainConfig, WorkspaceConfig, WorkspaceProjects,
};
use moon_project_constraints::{enforce_project_type_relationships, enforce_tag_relationships};
use moon_project_graph::{
    ExtendProjectEvent, ExtendProjectGraphEvent, ProjectGraph, ProjectGraphType, ProjectNode,
};
use moon_vcs::BoxedVcs;
use petgraph::prelude::*;
use petgraph::visit::IntoNodeReferences;
use rustc_hash::FxHashMap;
use starbase_events::Emitter;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, instrument, trace};

#[derive(Default)]
pub struct ProjectBuildData {
    alias: Option<String>,
    config: ProjectConfig,
    original_id: Option<Id>,
    source: WorkspaceRelativePathBuf,
}

pub struct WorkspaceBuilderContext<'app> {
    pub config_loader: &'app ConfigLoader,
    pub extend_project: Emitter<ExtendProjectEvent>,
    pub extend_project_graph: Emitter<ExtendProjectGraphEvent>,
    pub inherited_tasks: &'app InheritedTasksManager,
    pub toolchain_config: &'app ToolchainConfig,
    pub vcs: Option<Arc<BoxedVcs>>,
    pub working_dir: &'app Path,
    pub workspace_config: &'app WorkspaceConfig,
    pub workspace_root: &'app Path,
}

pub struct WorkspaceBuildResult {
    pub project_graph: ProjectGraph,
}

pub struct WorkspaceBuilder<'app> {
    context: Arc<WorkspaceBuilderContext<'app>>,

    /// Mapping of project IDs to associated data required for building
    /// the project itself. Currently we track the following:
    ///   - The alias, derived from manifests (`package.json`).
    ///   - Their `moon.yml` in the project root.
    ///   - Their file source location, relative from the workspace root.
    project_data: FxHashMap<Id, ProjectBuildData>,

    /// The project DAG.
    project_graph: ProjectGraphType,

    /// Projects that have explicitly renamed themselves with the `id` setting.
    /// Maps original ID to renamed ID.
    renamed_project_ids: FxHashMap<Id, Id>,

    /// The type of repository: monorepo or polyrepo.
    repo_type: RepoType,

    /// The root project ID (only if a monorepo).
    root_project_id: Option<Id>,
}

impl<'app> WorkspaceBuilder<'app> {
    pub async fn new(
        context: WorkspaceBuilderContext<'app>,
    ) -> miette::Result<WorkspaceBuilder<'app>> {
        debug!("Building project and task graphs");

        let mut graph = WorkspaceBuilder {
            context: Arc::new(context),
            project_data: FxHashMap::default(),
            project_graph: ProjectGraphType::default(),
            renamed_project_ids: FxHashMap::default(),
            repo_type: RepoType::Unknown,
            root_project_id: None,
        };

        graph.preload_build_data().await?;
        graph.determine_repo_type()?;

        Ok(graph)
    }

    /// Build the project graph and return a new structure.
    #[instrument(name = "build_workspace_graph", skip_all)]
    pub async fn build(self) -> miette::Result<WorkspaceBuildResult> {
        self.enforce_constraints()?;

        let project_nodes = self
            .project_data
            .into_iter()
            .map(|(id, data)| {
                (
                    id,
                    ProjectNode {
                        alias: data.alias,
                        index: NodeIndex::default(), // TODO
                        original_id: data.original_id,
                        source: data.source,
                    },
                )
            })
            .collect::<FxHashMap<_, _>>();

        let mut project_graph = ProjectGraph::new(
            self.project_graph,
            project_nodes,
            self.context.workspace_root,
        );

        project_graph.working_dir = self.context.working_dir.to_owned();

        Ok(WorkspaceBuildResult { project_graph })
    }

    /// Determine the repository type/structure based on the number of project
    /// sources, and where the point to.
    fn determine_repo_type(&mut self) -> miette::Result<()> {
        let single_project = self.project_data.len() == 1;
        let mut has_root_project = false;
        let mut root_project_id = None;

        for (id, build_data) in &self.project_data {
            if is_root_level_source(&build_data.source) {
                has_root_project = true;
                root_project_id = Some(id.to_owned());
                break;
            }
        }

        self.repo_type = match (single_project, has_root_project) {
            (true, _) => RepoType::Polyrepo,
            (false, true) => RepoType::MonorepoWithRoot,
            (false, false) => RepoType::Monorepo,
        };

        if self.repo_type == RepoType::MonorepoWithRoot {
            self.root_project_id = root_project_id;
        }

        Ok(())
    }

    /// Enforce project constraints and boundaries after all nodes have been inserted.
    #[instrument(skip_all)]
    fn enforce_constraints(&self) -> miette::Result<()> {
        debug!("Enforcing project constraints");

        let context = &self.context;
        let type_relationships = context
            .workspace_config
            .constraints
            .enforce_project_type_relationships;
        let tag_relationships = &context.workspace_config.constraints.tag_relationships;

        if !type_relationships && tag_relationships.is_empty() {
            return Ok(());
        }

        let default_scope = DependencyScope::Build;

        for (project_index, project) in self.project_graph.node_references() {
            let deps: Vec<_> = self
                .project_graph
                .neighbors_directed(project_index, Direction::Outgoing)
                .flat_map(|dep_index| {
                    self.project_graph.node_weight(dep_index).map(|dep| {
                        (
                            dep,
                            // Is this safe?
                            self.project_graph
                                .find_edge(project_index, dep_index)
                                .and_then(|ei| self.project_graph.edge_weight(ei))
                                .unwrap_or(&default_scope),
                        )
                    })
                })
                .collect();

            for (dep, dep_scope) in deps {
                if type_relationships {
                    enforce_project_type_relationships(project, dep, dep_scope)?;
                }

                for (source_tag, required_tags) in tag_relationships {
                    enforce_tag_relationships(project, source_tag, dep, required_tags)?;
                }
            }
        }

        Ok(())
    }

    /// Preload the graph with project sources from the workspace configuration.
    /// If globs are provided, walk the file system and gather sources.
    /// Then extend the graph with aliases, derived from all event subscribers.
    async fn preload_build_data(&mut self) -> miette::Result<()> {
        let context = self.context.clone();
        let mut globs = vec![];
        let mut sources = vec![];

        // Gather all project sources
        let mut add_sources = |map: &FxHashMap<Id, String>| {
            for (id, source) in map {
                sources.push((id.to_owned(), WorkspaceRelativePathBuf::from(source)));
            }
        };

        match &context.workspace_config.projects {
            WorkspaceProjects::Sources(map) => {
                add_sources(map);
            }
            WorkspaceProjects::Globs(list) => {
                globs.extend(list);
            }
            WorkspaceProjects::Both(cfg) => {
                globs.extend(&cfg.globs);
                add_sources(&cfg.sources);
            }
        };

        if !sources.is_empty() {
            debug!(
                sources = ?sources,
                "Using configured project sources",
            );
        }

        if !globs.is_empty() {
            debug!(
                globs = ?globs,
                "Locating projects with globs",
            );

            locate_projects_with_globs(&context, &globs, &mut sources)?;
        }

        // Load projects first
        self.load_project_build_data(sources)?;

        // Then load aliases and extend projects
        self.load_project_aliases().await?;

        Ok(())
    }

    async fn load_project_aliases(&mut self) -> miette::Result<()> {
        let context = &self.context;

        debug!("Extending project graph with aliases");

        let aliases = context
            .extend_project_graph
            .emit(ExtendProjectGraphEvent {
                sources: self
                    .project_data
                    .iter()
                    .map(|(id, build_data)| (id.to_owned(), build_data.source.to_owned()))
                    .collect(),
                workspace_root: context.workspace_root.to_owned(),
            })
            .await?
            .aliases;

        let mut dupe_aliases = FxHashMap::<String, Id>::default();

        for (id, alias) in aliases {
            let id = self.renamed_project_ids.get(&id).unwrap_or(&id);

            // Skip aliases that match its own ID
            if id == &alias {
                continue;
            }

            // Skip aliases that would override an ID
            if self.project_data.contains_key(alias.as_str()) {
                debug!(
                    "Skipping alias {} for project {} as it conflicts with the existing project {}",
                    color::label(&alias),
                    color::id(id),
                    color::id(&alias),
                );

                continue;
            }

            if let Some(existing_id) = dupe_aliases.get(&alias) {
                // Skip if the existing ID is already for this ID.
                // This scenario is possible when multiple platforms
                // extract the same aliases (Bun vs Node, etc).
                if existing_id == id {
                    continue;
                }

                return Err(WorkspaceBuilderError::DuplicateProjectAlias {
                    alias: alias.clone(),
                    old_id: existing_id.to_owned(),
                    new_id: id.clone(),
                }
                .into());
            }

            dupe_aliases.insert(alias.clone(), id.to_owned());

            self.project_data
                .get_mut(id)
                .expect("Project build data not found!")
                .alias = Some(alias);
        }

        Ok(())
    }

    fn load_project_build_data(&mut self, sources: ProjectsSourcesList) -> miette::Result<()> {
        let context = &self.context;
        let config_label = context.config_loader.get_debug_label("moon", false);
        let mut project_data: FxHashMap<Id, ProjectBuildData> = FxHashMap::default();
        let mut renamed_ids = FxHashMap::default();

        debug!("Loading projects");

        for (mut id, source) in sources {
            trace!(
                id = id.as_str(),
                "Attempting to load {} (optional)",
                color::file(source.join(&config_label))
            );

            let mut build_data = ProjectBuildData {
                config: context
                    .config_loader
                    .load_project_config_from_source(context.workspace_root, &source)?,
                source,
                ..Default::default()
            };

            // Track ID renames
            if let Some(new_id) = &build_data.config.id {
                if new_id != &id {
                    build_data.original_id = Some(id.clone());
                    renamed_ids.insert(id.clone(), new_id.to_owned());
                    id = new_id.to_owned();
                }
            }

            // Check for duplicate IDs
            if let Some(existing_data) = project_data.get(&id) {
                if existing_data.source != build_data.source {
                    return Err(WorkspaceBuilderError::DuplicateProjectId {
                        id: id.clone(),
                        old_source: existing_data.source.to_string(),
                        new_source: build_data.source.to_string(),
                    }
                    .into());
                }
            }

            // Otherwise persist the build data
            project_data.insert(id, build_data);
        }

        debug!("Loaded {} projects", project_data.len());

        self.project_data.extend(project_data);
        self.renamed_project_ids.extend(renamed_ids);

        Ok(())
    }
}
