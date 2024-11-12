use miette::IntoDiagnostic;
use moon_affected::Affected;
use moon_common::Id;
use moon_project::Project;
use moon_task::Task;
use moon_workspace_graph::WorkspaceGraph;
use serde::{Deserialize, Serialize};
use starbase::AppResult;
use std::{collections::BTreeMap, sync::Arc};
use tracing::{debug, trace};

#[derive(Default, Deserialize, Serialize)]
pub struct QueryProjectsOptions {
    pub alias: Option<String>,
    pub affected: Option<Affected>,
    pub id: Option<String>,
    pub json: bool,
    pub language: Option<String>,
    pub query: Option<String>,
    pub stack: Option<String>,
    pub source: Option<String>,
    pub tags: Option<String>,
    pub tasks: Option<String>,
    #[serde(rename = "type")]
    pub type_of: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct QueryProjectsResult {
    pub projects: Vec<Arc<Project>>,
    pub options: QueryProjectsOptions,
}

#[derive(Deserialize, Serialize)]
pub struct QueryTasksResult {
    pub tasks: BTreeMap<Id, BTreeMap<Id, Arc<Task>>>,
    pub options: QueryProjectsOptions,
}

fn convert_to_regex(field: &str, value: &Option<String>) -> AppResult<Option<regex::Regex>> {
    match value {
        Some(pattern) => {
            trace!(
                "Filtering projects \"{}\" by matching pattern \"{}\"",
                field,
                pattern
            );

            // case-insensitive by default
            let pat = regex::Regex::new(&format!("(?i){pattern}")).into_diagnostic()?;

            Ok(Some(pat))
        }
        None => Ok(None),
    }
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
    let stack_regex = convert_to_regex("stack", &options.stack)?;
    let source_regex = convert_to_regex("source", &options.source)?;
    let tags_regex = convert_to_regex("tags", &options.tags)?;
    let tasks_regex = convert_to_regex("tasks", &options.tasks)?;
    let type_regex = convert_to_regex("type", &options.type_of)?;
    let mut filtered = vec![];

    for project in workspace_graph.get_all_projects()? {
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

        if let Some(regex) = &type_regex {
            if !regex.is_match(&project.type_of.to_string()) {
                continue;
            }
        }

        filtered.push(project);
    }

    Ok(filtered)
}

pub async fn query_projects(
    workspace_graph: &WorkspaceGraph,
    options: &QueryProjectsOptions,
) -> AppResult<Vec<Arc<Project>>> {
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
