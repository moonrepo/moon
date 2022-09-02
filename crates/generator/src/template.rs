use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq)]
pub struct Template {
    pub name: String,
    pub root: PathBuf,
}
