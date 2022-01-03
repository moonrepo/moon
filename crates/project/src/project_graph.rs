use crate::constants::ROOT_NODE_ID;
use crate::errors::ProjectError;
use crate::project::Project;
use dep_graph::{DepGraph, Node};
use itertools::Itertools;
use moon_config::{GlobalProjectConfig, ProjectID};
use moon_logger::{color, debug, trace};
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct ProjectGraph {
    /// The global project configuration that all projects inherit from.
    /// Is loaded from `.moon/project.yml`.
    global_config: GlobalProjectConfig,

    /// A lightweight dependency graph, where each node is a project ID,
    /// and can depend on other project IDs.
    nodes: RefCell<HashMap<ProjectID, Node<ProjectID>>>,

    /// Projects that have been loaded into the graph.
    projects: RefCell<HashMap<ProjectID, Project>>,

    /// The mapping of projects by ID to a relative file system location.
    /// Is the `projects` setting in `.moon/workspace.yml`.
    projects_config: HashMap<ProjectID, String>,

    /// The workspace root, in which projects are relatively loaded from.
    workspace_root: PathBuf,
}

impl ProjectGraph {
    pub fn new(
        workspace_root: &Path,
        global_config: GlobalProjectConfig,
        projects_config: &HashMap<ProjectID, String>,
    ) -> ProjectGraph {
        debug!(
            target: "moon:project-graph",
            "Creating project graph with {} projects",
            projects_config.len(),
        );

        ProjectGraph {
            global_config,
            nodes: RefCell::new(HashMap::from([(
                ROOT_NODE_ID.to_owned(),
                Node::new(ROOT_NODE_ID.to_owned()),
            )])),
            projects: RefCell::new(HashMap::new()),
            projects_config: projects_config.clone(),
            workspace_root: workspace_root.to_path_buf(),
        }
    }

    /// Return a list of all configured project IDs in ascending order.
    pub fn ids(&self) -> std::vec::IntoIter<&ProjectID> {
        self.projects_config.keys().sorted()
    }

    /// Return a project with the associated ID. If the project
    /// has not been loaded, it will be loaded and inserted into the
    /// project graph. If the project does not exist or has been
    /// misconfigured, an error will be returned.
    pub fn get(&self, id: &str) -> Result<Project, ProjectError> {
        let mut projects = self.projects.borrow_mut();
        let mut nodes = self.nodes.borrow_mut();

        // Lazy load the project if it has not been
        self.load(&mut projects, &mut nodes, id)?;

        // TODO: Is it possible to not clone here???
        Ok(projects.get(id).unwrap().clone())
    }

    /// Return a list of project IDs that a project depends on,
    /// in the priority order in which they are depended on.
    pub fn get_dependencies_of(&self, project: &Project) -> Result<Vec<ProjectID>, ProjectError> {
        let mut nodes = vec![];

        self.extract_nodes(&mut nodes, &project.id)?;

        Ok(DepGraph::new(&nodes).into_iter().collect())
    }

    /// Return a list of project IDs that a project depends on,
    /// in ascending order.
    pub fn get_sorted_dependencies_of(
        &self,
        project: &Project,
    ) -> Result<Vec<ProjectID>, ProjectError> {
        let mut deps = self.get_dependencies_of(project)?;
        deps.sort();

        Ok(deps)
    }

    /// Recursively extract a list of nodes based on its dependency chain.
    fn extract_nodes(
        &self,
        nodes: &mut Vec<Node<ProjectID>>,
        id: &str,
    ) -> Result<(), ProjectError> {
        match self.nodes.borrow().get(id) {
            Some(node) => {
                for dep in node.deps() {
                    self.extract_nodes(nodes, dep)?;
                }

                nodes.push(node.clone());
            }
            None => return Err(ProjectError::UnconfiguredID(id.to_owned())),
        }

        Ok(())
    }

    /// Internal method for lazily loading a project and its
    /// dependencies into the graph.
    fn load(
        &self,
        projects: &mut RefMut<HashMap<ProjectID, Project>>,
        nodes: &mut RefMut<HashMap<ProjectID, Node<ProjectID>>>,
        id: &str,
    ) -> Result<(), ProjectError> {
        // Already loaded, abort early
        if projects.contains_key(id) || id == ROOT_NODE_ID {
            trace!(
                target: "moon:project-graph",
                "Project {} already exists in the project graph",
                color::id(id),
            );

            return Ok(());
        }

        trace!(
            target: "moon:project-graph",
            "Project {} does not exist in the project graph, attempting to load",
            color::id(id),
        );

        // Create project based on ID and location
        let location = match self.projects_config.get(id) {
            Some(path) => path,
            None => return Err(ProjectError::UnconfiguredID(String::from(id))),
        };

        let project = Project::new(id, location, &self.workspace_root, &self.global_config)?;
        let depends_on = project.get_dependencies();

        projects.insert(id.to_owned(), project);

        // Insert the project into the graph
        let mut node = Node::new(id.to_owned());

        if !depends_on.is_empty() {
            trace!(
                target: "moon:project-graph",
                "Adding dependencies {} to project {}",
                depends_on.clone().into_iter().map(|d| color::symbol(&d)).join(", "),
                color::id(id),
            );

            for dep in depends_on {
                // Ensure the dependent project is also loaded
                self.load(projects, nodes, dep.as_str())?;

                node.add_dep(dep);
            }
        }

        nodes.insert(id.to_owned(), node);

        Ok(())
    }
}
