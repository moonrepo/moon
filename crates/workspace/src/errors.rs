use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("Unable to determine workspace root. Please create a `{0}` configuration folder.")]
    MissingConfigDir(String), // dir name

    #[error("Unable to locate workspace configuration file. Please create a `{0}` file.")]
    MissingWorkspaceConfigFile(String), // dir + file name

    #[error("Unknown monolith workspace error.")]
    Unknown,
}
