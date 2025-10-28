pub mod changed_files;
pub mod projects;
pub mod tasks;

use clap::Subcommand;

pub(super) const HEADING_AFFECTED: &str = "Affected by";
pub(super) const HEADING_FILTERS: &str = "Filters";

#[derive(Clone, Debug, Subcommand)]
pub enum QueryCommands {
    #[command(
        name = "changed-files",
        about = "Query for changed files between revisions."
    )]
    ChangedFiles(changed_files::QueryChangedFilesArgs),

    #[command(
        name = "projects",
        about = "Query for projects within the project graph."
    )]
    Projects(projects::QueryProjectsArgs),

    #[command(name = "tasks", about = "Query for tasks, grouped by project.")]
    Tasks(tasks::QueryTasksArgs),
}
