use super::convert_to_regex;
use moon_affected::Affected;
use moon_common::{Id, color};
use moon_task::Task;
use moon_workspace_graph::WorkspaceGraph;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
use tracing::{debug, warn};

#[derive(Default, Deserialize, Serialize)]
pub struct QueryTasksOptions {
    pub affected: Option<Affected>,
    pub json: bool,
    pub query: Option<String>,

    // Filters
    pub id: Option<String>,
    pub command: Option<String>,
    // TODO: Remove in 2.0
    pub platform: Option<String>,
    pub project: Option<String>,
    pub script: Option<String>,
    pub toolchain: Option<String>,
    #[serde(rename = "type")]
    pub type_of: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct QueryTasksResult {
    pub tasks: BTreeMap<Id, BTreeMap<Id, Arc<Task>>>,
    pub options: QueryTasksOptions,
}

fn load_with_query(
    workspace_graph: &WorkspaceGraph,
    query: &str,
) -> miette::Result<Vec<Arc<Task>>> {
    workspace_graph.query_tasks(moon_query::build_query(query)?)
}

fn load_with_regex(
    workspace_graph: &WorkspaceGraph,
    options: &QueryTasksOptions,
) -> miette::Result<Vec<Arc<Task>>> {
    if options.platform.is_some() {
        warn!(
            "The {} option is deprecated, use {} instead",
            color::property("--platform"),
            color::property("--toolchain"),
        );
    }

    let id_regex = convert_to_regex("id", &options.id)?;
    let command_regex = convert_to_regex("command", &options.command)?;
    let platform_regex = convert_to_regex("platform", &options.platform)?;
    let project_regex = convert_to_regex("project", &options.project)?;
    let script_regex = convert_to_regex("script", &options.script)?;
    let mut toolchain_regex = convert_to_regex("toolchain", &options.toolchain)?;
    let type_regex = convert_to_regex("type", &options.type_of)?;
    let mut filtered = vec![];

    if toolchain_regex.is_none() {
        toolchain_regex = platform_regex;
    }

    for task in workspace_graph.get_tasks()? {
        if let Some(regex) = &id_regex {
            if !regex.is_match(&task.id) {
                continue;
            }
        }

        if let (Some(regex), Some(project_id)) = (&project_regex, task.target.get_project_id()) {
            if !regex.is_match(project_id.as_str()) {
                continue;
            }
        }

        if let Some(regex) = &command_regex {
            if !regex.is_match(&task.command) {
                continue;
            }
        }

        if let (Some(regex), Some(script)) = (&script_regex, &task.script) {
            if !regex.is_match(script) {
                continue;
            }
        }

        if let Some(regex) = &toolchain_regex {
            if !task.toolchains.iter().any(|tc| regex.is_match(tc)) {
                continue;
            }
        }

        if let Some(regex) = &type_regex {
            if !regex.is_match(&task.type_of.to_string()) {
                continue;
            }
        }

        filtered.push(task);
    }

    Ok(filtered)
}

pub async fn query_tasks(
    workspace_graph: &WorkspaceGraph,
    options: &QueryTasksOptions,
) -> miette::Result<Vec<Arc<Task>>> {
    debug!("Querying for tasks");

    let mut tasks = if let Some(query) = &options.query {
        load_with_query(workspace_graph, query)?
    } else {
        load_with_regex(workspace_graph, options)?
    };

    if let Some(affected) = &options.affected {
        debug!("Filtering based on affected");

        tasks = tasks
            .into_iter()
            .filter_map(|task| {
                if affected.is_task_affected(&task.target) {
                    Some(task)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
    }

    Ok(tasks)
}
