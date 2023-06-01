use crate::queries::touched_files::{
    query_touched_files, QueryTouchedFilesOptions, QueryTouchedFilesResult,
};
use moon::generate_project_graph;
use moon_common::Id;
use moon_error::MoonError;
use moon_logger::{debug, trace};
use moon_project::Project;
use moon_task::{Task, TouchedFilePaths};
use moon_utils::{is_ci, regex};
use moon_workspace::Workspace;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use starbase::AppResult;
use std::{
    collections::BTreeMap,
    io::{stdin, IsTerminal, Read},
    path::PathBuf,
};

const LOG_TARGET: &str = "moon:query:projects";

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct QueryProjectsOptions {
    pub alias: Option<String>,
    pub affected: bool,
    pub id: Option<String>,
    pub json: bool,
    pub language: Option<String>,
    pub query: Option<String>,
    pub source: Option<String>,
    pub tags: Option<String>,
    pub tasks: Option<String>,
    pub type_of: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct QueryProjectsResult {
    pub projects: Vec<Project>,
    pub options: QueryProjectsOptions,
}

#[derive(Deserialize, Serialize)]
pub struct QueryTasksResult {
    pub tasks: FxHashMap<Id, BTreeMap<Id, Task>>,
    pub options: QueryProjectsOptions,
}

fn convert_to_regex(field: &str, value: &Option<String>) -> AppResult<Option<regex::Regex>> {
    match value {
        Some(pattern) => {
            trace!(
                target: LOG_TARGET,
                "Filtering projects \"{}\" by matching pattern \"{}\"",
                field,
                pattern
            );

            // case-insensitive by default
            Ok(Some(regex::create_regex(format!("(?i){pattern}"))?))
        }
        None => Ok(None),
    }
}

async fn load_touched_files(workspace: &Workspace) -> AppResult<TouchedFilePaths> {
    let mut buffer = String::new();

    // Only read piped data when stdin is not a TTY,
    // otherwise the process will hang indefinitely waiting for EOF.
    if !stdin().is_terminal() {
        stdin().read_to_string(&mut buffer).map_err(MoonError::Io)?;
    }

    // If piped via stdin, parse and use it
    if !buffer.is_empty() {
        // As JSON
        if buffer.starts_with('{') {
            let result: QueryTouchedFilesResult =
                serde_json::from_str(&buffer).map_err(|e| MoonError::Generic(e.to_string()))?;

            return Ok(result.files);

            // As lines
        } else {
            let files = FxHashSet::from_iter(buffer.split('\n').map(PathBuf::from));

            return Ok(files);
        }
    }

    query_touched_files(
        workspace,
        &mut QueryTouchedFilesOptions {
            local: !is_ci(),
            ..QueryTouchedFilesOptions::default()
        },
    )
    .await
}

pub async fn query_projects(
    workspace: &mut Workspace,
    options: &QueryProjectsOptions,
) -> AppResult<Vec<Project>> {
    debug!(target: LOG_TARGET, "Querying for projects");

    let project_graph = generate_project_graph(workspace).await?;
    let touched_files = if options.affected {
        load_touched_files(workspace).await?
    } else {
        FxHashSet::default()
    };

    // When a MQL input is provided, it takes full precedence over option args
    if let Some(query) = &options.query {
        let projects = project_graph
            .query(&moon_query::build_query(query)?)?
            .into_iter()
            .filter_map(|project| {
                if options.affected && !project.is_affected(&touched_files) {
                    return None;
                }

                Some(project.to_owned())
            })
            .collect::<Vec<_>>();

        return Ok(projects);
    }

    let alias_regex = convert_to_regex("alias", &options.alias)?;
    let id_regex = convert_to_regex("id", &options.id)?;
    let language_regex = convert_to_regex("language", &options.language)?;
    let source_regex = convert_to_regex("source", &options.source)?;
    let tags_regex = convert_to_regex("tags", &options.tags)?;
    let tasks_regex = convert_to_regex("tasks", &options.tasks)?;
    let type_regex = convert_to_regex("type", &options.type_of)?;
    let mut projects = vec![];

    for project in project_graph.get_all()? {
        if options.affected && !project.is_affected(&touched_files) {
            continue;
        }

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
            if !regex.is_match(&project.source) {
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
            let has_task = project.tasks.keys().any(|task_id| regex.is_match(task_id));

            if !has_task {
                continue;
            }
        }

        if let Some(regex) = &language_regex {
            if !regex.is_match(&project.language.to_string()) {
                continue;
            }
        }

        if let Some(regex) = &type_regex {
            if !regex.is_match(&project.type_of.to_string()) {
                continue;
            }
        }

        projects.push(project.to_owned());
    }

    Ok(projects)
}
