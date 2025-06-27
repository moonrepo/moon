use super::convert_to_regex;
use moon_affected::Affected;
use moon_project::Project;
use moon_workspace_graph::{GraphConnections, WorkspaceGraph};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Default, Deserialize, Serialize)]
pub struct QueryProjectsOptions {
    pub affected: Option<Affected>,
    pub json: bool,
    pub query: Option<String>,

    // Filters
    pub alias: Option<String>,
    pub id: Option<String>,
    pub language: Option<String>,
    #[serde(alias = "type")]
    pub layer: Option<String>,
    pub stack: Option<String>,
    pub source: Option<String>,
    pub tags: Option<String>,
    pub tasks: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct QueryProjectsResult {
    pub projects: Vec<Arc<Project>>,
    pub options: QueryProjectsOptions,
}

fn load_with_query(
    workspace_graph: &WorkspaceGraph,
    query: &str,
) -> miette::Result<Vec<Arc<Project>>> {
    workspace_graph.query_projects(moon_query::build_query(query)?)
}

fn load_with_regex(
    workspace_graph: &WorkspaceGraph,
    options: &QueryProjectsOptions,
) -> miette::Result<Vec<Arc<Project>>> {
    let alias_regex = convert_to_regex("alias", &options.alias)?;
    let id_regex = convert_to_regex("id", &options.id)?;
    let language_regex = convert_to_regex("language", &options.language)?;
    let layer_regex = convert_to_regex("layer", &options.layer)?;
    let stack_regex = convert_to_regex("stack", &options.stack)?;
    let source_regex = convert_to_regex("source", &options.source)?;
    let tags_regex = convert_to_regex("tags", &options.tags)?;
    let tasks_regex = convert_to_regex("tasks", &options.tasks)?;
    let mut filtered = vec![];

    for project_id in workspace_graph.projects.get_node_keys() {
        // Include tasks for JSON output
        let project = workspace_graph.get_project_with_tasks(project_id)?;

        if let Some(regex) = &id_regex {
            if !regex.is_match(&project.id) {
                continue;
            }
        }

        if let Some(regex) = &alias_regex {
            if let Some(alias) = &project.alias {
                if !regex.is_match(alias) {
                    continue;
                }
            }
        }

        if let Some(regex) = &source_regex {
            if !regex.is_match(project.source.as_str()) {
                continue;
            }
        }

        if let Some(regex) = &tags_regex {
            let has_tag = project.config.tags.iter().any(|tag| regex.is_match(tag));

            if !has_tag {
                continue;
            }
        }

        if let Some(regex) = &tasks_regex {
            let has_task = project
                .task_targets
                .iter()
                .any(|target| regex.is_match(&target.task_id));

            if !has_task {
                continue;
            }
        }

        if let Some(regex) = &language_regex {
            if !regex.is_match(&project.language.to_string()) {
                continue;
            }
        }

        if let Some(regex) = &stack_regex {
            if !regex.is_match(&project.stack.to_string()) {
                continue;
            }
        }

        if let Some(regex) = &layer_regex {
            if !regex.is_match(&project.layer.to_string()) {
                continue;
            }
        }

        filtered.push(Arc::new(project));
    }

    Ok(filtered)
}

pub async fn query_projects(
    workspace_graph: &WorkspaceGraph,
    options: &QueryProjectsOptions,
) -> miette::Result<Vec<Arc<Project>>> {
    debug!("Querying for projects");

    let mut projects = if let Some(query) = &options.query {
        load_with_query(workspace_graph, query)?
    } else {
        load_with_regex(workspace_graph, options)?
    };

    if let Some(affected) = &options.affected {
        debug!("Filtering based on affected");

        projects = projects
            .into_iter()
            .filter_map(|project| {
                if affected.is_project_affected(&project.id) {
                    Some(project)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
    }

    Ok(projects)
}
