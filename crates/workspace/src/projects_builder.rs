use crate::workspace_builder::WorkspaceBuilderContext;
use crate::workspace_builder_error::WorkspaceBuilderError;
use daggy::{Dag, NodeIndex};
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_config::{DependencyScope, ProjectConfig, ProjectDependencyConfig, finalize_config};
use moon_pdk_api::{ExtendProjectGraphInput, ExtendProjectGraphOutput, ExtendProjectOutput};
use moon_project::{Project, ProjectAlias};
use moon_project_builder::{ProjectBuilder, ProjectBuilderContext};
use moon_task_graph::NodeState;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ProjectBuildData {
    /// Map of aliases to the plugin that provided them.
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub aliases: FxHashMap<String, Id>,

    #[serde(skip)]
    pub config: Option<ProjectConfig>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<ExtendProjectOutput>,

    // Only used for renaming!
    #[serde(skip)]
    pub id: Option<Id>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_index: Option<NodeIndex>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_id: Option<Id>,

    pub source: WorkspaceRelativePathBuf,
}

impl ProjectBuildData {
    pub fn rename_id_if_configured(&mut self) -> Option<(Id, Id)> {
        if let Some(old_id) = &self.id
            && let Some(config) = &self.config
            && let Some(new_id) = &config.id
            && new_id != old_id
        {
            return Some((old_id.to_owned(), new_id.to_owned()));
        }

        None
    }

    pub fn resolve_id(id_or_alias: &str, project_data: &FxHashMap<Id, ProjectBuildData>) -> Id {
        if project_data.contains_key(id_or_alias) {
            Id::raw(id_or_alias)
        } else {
            match project_data.iter().find_map(|(id, build_data)| {
                if build_data.aliases.contains_key(id_or_alias) {
                    Some(id)
                } else {
                    None
                }
            }) {
                Some(project_id) => project_id.to_owned(),
                None => Id::raw(id_or_alias),
            }
        }
    }
}

pub enum ProjectBuildEvent {
    Node(Project),
    Edge(Id, Id, DependencyScope),
}

pub fn load_project_build_data(
    context: Arc<WorkspaceBuilderContext>,
    id: Id,
    source: WorkspaceRelativePathBuf,
) -> miette::Result<ProjectBuildData> {
    let config = context
        .config_loader
        .load_project_config_from_source(&context.workspace_root, &source)?;

    Ok(ProjectBuildData {
        config: Some(config),
        id: Some(id),
        source,
        ..Default::default()
    })
}

pub async fn extend_project_build_data_with_plugins(
    context: Arc<WorkspaceBuilderContext>,
    sources: BTreeMap<Id, String>,
) -> miette::Result<Vec<(Id, ExtendProjectGraphOutput, bool)>> {
    let mut outputs = vec![];

    // From toolchains
    for result in context
        .toolchain_registry
        .extend_project_graph_all(|registry, toolchain| ExtendProjectGraphInput {
            context: registry.create_context(),
            project_sources: sources.clone(),
            toolchain_config: registry.create_config(&toolchain.id),
            ..Default::default()
        })
        .await?
    {
        outputs.push((result.id, result.output, true));
    }

    // From extensions
    for result in context
        .extension_registry
        .extend_project_graph_all(|registry, extension| ExtendProjectGraphInput {
            context: registry.create_context(),
            project_sources: sources.clone(),
            extension_config: registry.create_config(&extension.id),
            ..Default::default()
        })
        .await?
    {
        outputs.push((result.id, result.output, false));
    }

    Ok(outputs)
}

pub async fn build_project(
    context: Arc<WorkspaceBuilderContext>,
    build_data: ProjectBuildData,
    id: Id,
    root_id: Option<Id>,
    monorepo: bool,
    tx: mpsc::Sender<ProjectBuildEvent>,
) -> miette::Result<()> {
    if !build_data.source.to_path(&context.workspace_root).exists() {
        return Err(
            WorkspaceBuilderError::MissingProjectAtSource(build_data.source.to_string()).into(),
        );
    }

    let mut builder = ProjectBuilder::new(
        &id,
        &build_data.source,
        ProjectBuilderContext {
            config_loader: &context.config_loader,
            enabled_toolchains: &context.enabled_toolchains,
            monorepo,
            root_project_id: root_id.as_ref(),
            toolchains_config: &context.toolchains_config,
            toolchain_registry: context.toolchain_registry.clone(),
            workspace_root: &context.workspace_root,
        },
    )?;

    // Inherit configs and tasks
    if let Some(config) = build_data.config {
        builder.inherit_local_config(&config).await?;
    } else {
        builder.load_local_config().await?;
    }

    builder.inherit_global_configs(&context.inherited_tasks)?;

    // Inherit from build data and plugins (toolchains, etc)
    for extended_data in build_data.extensions {
        for dep_config in extended_data.dependencies {
            builder.extend_with_dependency(ProjectDependencyConfig {
                id: dep_config.id,
                scope: dep_config.scope,
                ..Default::default()
            });
        }

        for (task_id, task_config) in extended_data.tasks {
            builder.extend_with_task(task_id, finalize_config(task_config)?);
        }
    }

    // Inherit aliases before building in case the project
    // references itself in tasks or dependencies
    builder.set_aliases(
        build_data
            .aliases
            .into_iter()
            .map(|(alias, plugin)| ProjectAlias { alias, plugin })
            .collect(),
    );

    let project = builder.build().await?;

    // Send an event for each project-to-project relationship,
    // but don't link the root project to anything
    for dep_config in &project.dependencies {
        if !dep_config.is_root_scope() {
            tx.send(ProjectBuildEvent::Edge(
                id.clone(),
                dep_config.id.clone(),
                dep_config.scope,
            ))
            .await
            .expect("TODO");
        }
    }

    // Send a final event for the project itself
    tx.send(ProjectBuildEvent::Node(project))
        .await
        .expect("TODO");

    Ok(())
}

pub fn get_or_insert_project_node(
    id: &Id,
    graph: &mut Dag<NodeState<Project>, DependencyScope>,
    indexes: &mut FxHashMap<Id, NodeIndex>,
) -> NodeIndex {
    if let Some(index) = indexes.get(id) {
        *index
    } else {
        let index = graph.add_node(NodeState::Loading);
        indexes.insert(id.to_owned(), index);
        index
    }
}

pub fn insert_or_update_project_node(
    project: Project,
    graph: &mut Dag<NodeState<Project>, DependencyScope>,
    indexes: &mut FxHashMap<Id, NodeIndex>,
) {
    // Project node may have been inserted through an edge first,
    // so we need to update the state from loading to loaded
    if let Some(index) = indexes.get(&project.id)
        && let Some(node) = graph.node_weight_mut(*index)
    {
        *node = NodeState::Loaded(project);
    }
    // Otherwise the node was inserted first, so we can set as loaded
    else {
        indexes.insert(
            project.id.clone(),
            graph.add_node(NodeState::Loaded(project)),
        );
    }
}
