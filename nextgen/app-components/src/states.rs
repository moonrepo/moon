use starbase::State;
use std::path::PathBuf;

#[derive(Debug, State)]
pub struct MoonDir(pub PathBuf);

#[derive(Debug, State)]
pub struct WorkingDir(pub PathBuf);

#[derive(Debug, State)]
pub struct WorkspaceRoot(pub PathBuf);
