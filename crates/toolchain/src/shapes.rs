use std::path::PathBuf;

#[derive(Debug)]
pub struct DependenciesWorkspace {
    pub root: PathBuf,
    pub members: Vec<String>,
}
