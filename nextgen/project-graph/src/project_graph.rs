use moon_common::Id;
use moon_config::DependencyScope;
use moon_project::Project;
use std::path::Path;

use petgraph::graph::{DiGraph, NodeIndex};

pub type GraphType = DiGraph<Project, DependencyScope>;

pub struct ProjectGraph {}

impl ProjectGraph {
    // pub fn dependencies_of(&self, project: &Project) -> miette::Result<Vec<&Id>> {}

    // pub fn dependents_of(&self, project: &Project) -> miette::Result<Vec<&Id>> {}

    // pub fn get(&self, alias_or_id: &str) -> miette::Result<&Project> {}

    // pub fn get_all(&self) -> miette::Result<Vec<&Project>> {}

    // pub fn get_from_path<P: AsRef<Path>>(&self, starting_file: P) -> miette::Result<&Project> {}

    // pub fn ids(&self) -> Vec<&Id> {}
}
