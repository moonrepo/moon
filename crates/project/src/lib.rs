mod errors;
mod project;

pub use errors::ProjectError;
pub use monolith_config::project::{FileGroups, ProjectID};
pub use project::{Project, ProjectGraph, ProjectsMap};
