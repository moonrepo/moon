use moon_logger::{debug, trace};
use moon_project::Project;
use moon_utils::regex;
use moon_workspace::{Workspace, WorkspaceError};
use serde::{Deserialize, Serialize};

const TARGET: &str = "moon:query:projects";

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct QueryProjectsOptions {
    pub id: Option<String>,
    pub source: Option<String>,
    pub tasks: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct QueryProjectsResult {
    pub projects: Vec<Project>,
    pub options: QueryProjectsOptions,
}

pub async fn query_projects(
    workspace: &Workspace,
    options: &QueryProjectsOptions,
) -> Result<Vec<Project>, WorkspaceError> {
    debug!(target: TARGET, "Querying for projects");

    let mut projects = vec![];

    let id_regex = match &options.id {
        Some(pattern) => {
            trace!(
                target: TARGET,
                "Filtering projects based on ID pattern \"{}\"",
                pattern
            );

            Some(regex::create_regex(pattern)?)
        }
        None => None,
    };

    let source_regex = match &options.source {
        Some(pattern) => {
            trace!(
                target: TARGET,
                "Filtering projects based on source path pattern \"{}\"",
                pattern
            );

            Some(regex::create_regex(pattern)?)
        }
        None => None,
    };

    let tasks_regex = match &options.tasks {
        Some(pattern) => {
            trace!(
                target: TARGET,
                "Filtering projects that have tasks matching pattern \"{}\"",
                pattern
            );

            Some(regex::create_regex(pattern)?)
        }
        None => None,
    };

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

        projects.push(project);
    }

    Ok(projects)
}
