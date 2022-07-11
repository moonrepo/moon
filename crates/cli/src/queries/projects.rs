use moon_logger::{debug, trace};
use moon_project::Project;
use moon_utils::regex;
use moon_workspace::{Workspace, WorkspaceError};
use serde::{Deserialize, Serialize};

const TARGET: &str = "moon:query:projects";

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct QueryProjectsOptions {
    pub id: Option<String>,
    pub language: Option<String>,
    pub source: Option<String>,
    pub tasks: Option<String>,
    pub type_of: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct QueryProjectsResult {
    pub projects: Vec<Project>,
    pub options: QueryProjectsOptions,
}

fn convert_to_regex(
    field: &str,
    value: &Option<String>,
) -> Result<Option<regex::Regex>, WorkspaceError> {
    match value {
        Some(pattern) => {
            trace!(
                target: TARGET,
                "Filtering projects \"{}\" by matching pattern \"{}\"",
                field,
                pattern
            );

            // case-insensitive by default
            Ok(Some(regex::create_regex(&format!("(?i){}", pattern))?))
        }
        None => Ok(None),
    }
}

pub async fn query_projects(
    workspace: &Workspace,
    options: &QueryProjectsOptions,
) -> Result<Vec<Project>, WorkspaceError> {
    debug!(target: TARGET, "Querying for projects");

    let mut projects = vec![];
    let id_regex = convert_to_regex("id", &options.id)?;
    let language_regex = convert_to_regex("language", &options.language)?;
    let source_regex = convert_to_regex("source", &options.source)?;
    let tasks_regex = convert_to_regex("tasks", &options.tasks)?;
    let type_regex = convert_to_regex("type", &options.type_of)?;

    for project_id in workspace.projects.ids() {
        if let Some(regex) = &id_regex {
            if !regex.is_match(&project_id) {
                continue;
            }
        }

        let project = workspace.projects.load(&project_id)?;

        if let Some(regex) = &source_regex {
            if !regex.is_match(&project.source) {
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
            if !regex.is_match(&project.config.language.to_string()) {
                continue;
            }
        }

        if let Some(regex) = &type_regex {
            if !regex.is_match(&project.config.type_of.to_string()) {
                continue;
            }
        }

        projects.push(project);
    }

    Ok(projects)
}
