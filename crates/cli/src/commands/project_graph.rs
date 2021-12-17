use crate::helpers::{print_list, safe_exit};
use itertools::Itertools;
use monolith_workspace::Workspace;

enum ProjectExitCodes {
    UnknownProject = 1,
}

pub async fn project_graph(
    workspace: &Workspace,
    dot: &bool,
    json: &bool,
) -> Result<(), clap::Error> {
    let projects = workspace.load_projects().unwrap(); // TODO error
    let graph = workspace.create_project_graph(&projects);

    // TODO tree output?
}
