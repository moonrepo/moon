use crate::errors::ProjectError;
use crate::project::Project;
use itertools::Itertools;
use monolith_config::{GlobalProjectConfig, ProjectID};
use solvent::DepGraph;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ProjectGraph {
    /// The global project configuration that all projects inherit from.
    /// Is loaded from `.monolith/project.yml`.
    global_config: GlobalProjectConfig,

    /// A lightweight dependency graph, where each node is a project ID,
    /// which can depend on other project IDs.
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
        graph.register_node("(root)".to_owned());

        ProjectGraph {
            global_config,
            graph: RefCell::new(graph),
            projects: RefCell::new(HashMap::new()),
            projects_config: projects_config.clone(),
            workspace_dir: workspace_dir.to_path_buf(),
        }
    }

    pub fn ids(&self) -> std::vec::IntoIter<&String> {
        self.projects_config.keys().sorted()
    }

    pub fn get(&self, id: &str) -> Result<Ref<Project>, ProjectError> {
        // Return project early if already loaded, and place
        // in a block so the borrow scope is dropped early
        {
            let projects = self.projects.borrow();

            if projects.contains_key(id) {
                return Ok(Ref::map(projects, |p| p.get(id).unwrap()));
            }
        }

        // Create and store project based on ID and location.
        // We also do this in a block to drop borrow mut scope.
        {
            let location = match self.projects_config.get(id) {
                Some(path) => path,
                None => return Err(ProjectError::UnconfiguredID(String::from(id))),
            };

            self.projects.borrow_mut().insert(
                id.to_owned(),
                Project::new(id, location, &self.workspace_dir, &self.global_config)?,
            );
        }

        self.get(id)
    }

    // pub fn load(&self, id: &str) -> Result<Ref<Project>, ProjectError> {
    //     // Return project early if already loaded, and place
    //     // in a block so the borrow scope is dropped early
    //     {
    //         let projects = self.projects.borrow();

    //         if projects.contains_key(id) {
    //             return Ok(Ref::map(projects, |p| p.get(id).unwrap()));
    //         }
    //     }

    //     // Create project based on ID and location
    //     let location = match self.projects_config.get(id) {
    //         Some(path) => path,
    //         None => return Err(ProjectError::UnconfiguredID(String::from(id))),
    //     };

    //     let project = Project::new(id, location, &self.workspace_dir, &self.global_config)?;

    //     println!("{} {} {:#?}", id, location, project);

    //     // Determine dependency list
    //     let mut depends_on = vec!["(root)".to_owned()];

    //     if project.config.is_some() {
    //         let config = project.config.as_ref().unwrap();

    //         depends_on.extend_from_slice(config.depends_on.as_ref().unwrap_or(&vec![]));
    //     }

    //     println!("depends on {:?}", depends_on);

    //     // Insert the project into the graph
    //     {
    //         self.save_to_graph(id, project, depends_on);
    //     }

    //     self.get(id)
    // }

    fn save_to_graph(&self, id: &str, project: Project, depends_on: Vec<String>) {
        let mut projects = self.projects.borrow_mut();

        projects.insert(id.to_owned(), project);

        println!("1");

        let mut graph = self.graph.borrow_mut();

        println!("2");

        graph.register_node(id.to_owned());
        graph.register_dependencies(id.to_owned(), depends_on);

        println!("3");
    }

    // pub fn get(&self, id: String) -> Result<Ref<Project>, ProjectError> {
    //     let key = id.as_str();
    //     let projects = self.projects.borrow();

    //     // Return project early if already loaded
    //     if projects.contains_key(key) {
    //         return Ok(Ref::map(projects, |p| p.get(key).unwrap()));
    //     }

    //     // Create project based on ID and location
    //     let location = match self.projects_config.get(key) {
    //         Some(path) => path,
    //         None => return Err(ProjectError::UnconfiguredID(id)),
    //     };

    //     let project = Project::new(
    //         id.as_str(),
    //         location,
    //         &self.workspace_dir,
    //         &self.global_config,
    //     )?;

    //     println!("{} {} {:#?}", id, location, project);

    //     // Determine dependency list
    //     let mut depends_on = vec!["(root)".to_owned()];

    //     if project.config.is_some() {
    //         let config = project.config.as_ref().unwrap();

    //         depends_on.extend_from_slice(config.depends_on.as_ref().unwrap_or(&vec![]));
    //     }

    //     println!("depends on {:?}", depends_on);

    //     // Insert the project into the graph
    //     // self.projects.borrow_mut().insert(id.to_owned(), project);

    //     println!("1");

    //     let mut graph = self.graph.borrow_mut();

    //     println!("2");

    //     graph.register_node(id.clone());
    //     graph.register_dependencies(id.clone(), depends_on);

    //     println!("3");

    //     self.get(id)
    // }

    // pub fn get(&mut self, id: String) -> Result<&Project, ProjectError> {
    //     let key = id.as_str();

    //     // Avoid loading again
    //     if self.projects.contains_key(key) {
    //         return Ok(self.projects.get(key).unwrap());
    //     }

    //     let location = match self.projects_config.get(key) {
    //         Some(path) => path,
    //         None => return Err(ProjectError::UnconfiguredID(id)),
    //     };

    //     let project = Project::new(
    //         id.as_str(),
    //         location,
    //         &self.workspace_dir,
    //         &self.global_config,
    //     )?;

    //     let mut depends_on = vec!["(root)".to_owned()];

    //     if project.config.is_some() {
    //         let config = project.config.as_ref().unwrap();

    //         depends_on.extend_from_slice(config.depends_on.as_ref().unwrap_or(&vec![]));
    //     }

    //     self.projects.insert(id.to_owned(), project);
    //     self.graph.register_node(id.clone());
    //     self.graph.register_dependencies(id.clone(), depends_on);

    //     Ok(self.projects.get(key).unwrap())
    // }
}
