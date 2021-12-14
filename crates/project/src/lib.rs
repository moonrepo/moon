mod errors;
mod graph;
mod project;

pub use errors::ProjectError;
pub use graph::ProjectGraph;
use monolith_config::project::ProjectID;
pub use project::Project;
use std::collections::HashMap;

pub type ProjectsMap = HashMap<ProjectID, Project>;
