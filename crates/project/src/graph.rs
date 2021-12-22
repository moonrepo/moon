use crate::constants::ROOT_NODE_ID;
use crate::errors::ProjectError;
use crate::project::Project;
use itertools::Itertools;
use monolith_config::{GlobalProjectConfig, ProjectID};
use solvent::DepGraph;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ProjectGraph {
    /// The global project configuration that all projects inherit from.
    /// Is loaded from `.monolith/project.yml`.
    global_config: GlobalProjectConfig,

    /// A lightweight dependency graph, where each node is a project ID,
    /// and can depend on other project IDs.
    graph: RefCell<DepGraph<String>>,

    /// Projects that have been loaded into the graph.
    projects: RefCell<HashMap<ProjectID, Project>>,

    /// The mapping of projects by ID to a relative file system location.
    /// Is the `projects` setting in `.monolith/workspace.yml`.
    projects_config: HashMap<ProjectID, String>,

    /// The workspace root, in which projects are relatively loaded from.
    workspace_dir: PathBuf,
}

impl ProjectGraph {
    pub fn new(
        workspace_dir: &Path,
        global_config: GlobalProjectConfig,
        projects_config: &HashMap<ProjectID, String>,
    ) -> ProjectGraph {
        let mut graph = DepGraph::new();
        graph.register_node(ROOT_NODE_ID.to_owned());

        ProjectGraph {
            global_config,
            graph: RefCell::new(graph),
            projects: RefCell::new(HashMap::new()),
            projects_config: projects_config.clone(),
            workspace_dir: workspace_dir.to_path_buf(),
        }
    }

    /// Returns a list of all project IDs that have been configured,
    /// in ascending sorted order.
    pub fn ids(&self) -> std::vec::IntoIter<&String> {
        self.projects_config.keys().sorted()
    }

    /// Returns a project with the associated ID. If the project
    /// has not been loaded, it will be loaded and inserted into the
    /// project graph. If the project does not exist or has been
    /// misconfigured, an error will be returned.
    pub fn get(&self, id: &str) -> Result<Project, ProjectError> {
        let mut projects = self.projects.borrow_mut();
        let mut graph = self.graph.borrow_mut();

        // Lazy load the project if it has not been
        self.load(&mut projects, &mut graph, id)?;

        // TODO: Is it possible to not clone here???
        Ok(projects.get(id).unwrap().clone())
    }

    /// Internal method for lazily loading a project and its
    /// dependencies into the graph.
    fn load(
        &self,
        projects: &mut RefMut<HashMap<ProjectID, Project>>,
        graph: &mut RefMut<DepGraph<String>>,
        id: &str,
    ) -> Result<(), ProjectError> {
        // Already loaded, abort early
        if projects.contains_key(id) || id == ROOT_NODE_ID {
            return Ok(());
        }

        // Create project based on ID and location
        let location = match self.projects_config.get(id) {
            Some(path) => path,
            None => return Err(ProjectError::UnconfiguredID(String::from(id))),
        };

        let project = Project::new(id, location, &self.workspace_dir, &self.global_config)?;
        let depends_on = project.get_dependencies();

        projects.insert(id.to_owned(), project);

        // Insert the project into the graph
        graph.register_node(id.to_owned());

        for dep in depends_on {
            // Ensure the dependent project is also loaded
            self.load(projects, graph, dep.as_str())?;

            graph.register_dependency(id.to_owned(), dep);
        }

        Ok(())
    }
}
