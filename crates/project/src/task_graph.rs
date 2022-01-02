use crate::errors::ProjectError;
use crate::project_graph::ProjectGraph;
use crate::target::Target;
use crate::task::Task;
use crate::types::TouchedFilePaths;
use dep_graph::{DepGraph, Node};
use moon_config::TargetID;
use moon_logger::{color, debug, trace};
use std::collections::HashMap;

#[derive(Default)]
pub struct TaskGraph {
    nodes: HashMap<TargetID, Node<TargetID>>,

    tasks: HashMap<TargetID, Task>,
}

impl TaskGraph {
    pub fn new(
        projects: &ProjectGraph,
        touched_files: &TouchedFilePaths,
        target: TargetID,
    ) -> Result<Self, ProjectError> {
        debug!(
            target: "moon:task-graph",
            "Creating task graph, starting with target {}",
           color::id(&target),
        );

        let mut graph = TaskGraph::default();
        graph.load(projects, touched_files, target, None)?;

        Ok(graph)
    }

    fn load(
        &mut self,
        projects: &ProjectGraph,
        touched_files: &TouchedFilePaths,
        target: TargetID,
        parent_node: Option<&mut Node<TargetID>>,
    ) -> Result<(), ProjectError> {
        if self.nodes.contains_key(&target) {
            return Ok(());
        }

        trace!(
            target: "moon:task-graph",
            "Target {} does not exist in the task graph, attempting to load",
            color::id(&target),
        );

        let (project_id, task_id) = Target::parse(&target)?;

        // Validate project first
        let project = projects.get(&project_id)?;

        if !project.is_affected(touched_files) {
            trace!(
                target: "moon:task-graph",
                "Project {} not affected based on touched files, skipping",
                color::id(&project_id),
            );

            return Ok(());
        }

        // Validate task exists for project
        let task = match project.tasks.get(&task_id) {
            Some(t) => t,
            None => {
                return Err(ProjectError::UnconfiguredTask(task_id, project_id));
            }
        };

        if !task.is_affected(&project.dir, touched_files)? {
            trace!(
                target: "moon:task-graph",
                "Project {} task {} not affected based on touched files, skipping",
                color::id(&project_id),
                color::id(&task_id),
            );

            return Ok(());
        }

        // Add task to graph
        self.tasks.insert(target.clone(), task.clone());

        // Add dependencies

        let mut node = Node::new(target.clone());

        if !task.deps.is_empty() {
            let dep_names: Vec<String> = task
                .deps
                .clone()
                .into_iter()
                .map(|d| color::symbol(&d))
                .collect();

            trace!(
                target: "moon:task-graph",
                "Adding dependencies {} from target {}",
                dep_names.join(", "),
                color::id(&target),
            );

            for dep_target in &task.deps {
                self.load(projects, touched_files, dep_target.clone(), Some(&mut node))?;
            }
        }

        self.nodes.insert(target.clone(), node);

        // Link back to parent
        if let Some(parent) = parent_node {
            parent.add_dep(target);
        }

        Ok(())
    }

    pub fn graph(&self) -> Vec<&Task> {
        let nodes = self
            .nodes
            .iter()
            .map(|(_, node)| node.clone())
            .collect::<Vec<Node<TargetID>>>();
        let mut tasks: Vec<&Task> = vec![];

        DepGraph::new(&nodes).into_iter().for_each(|target| {
            tasks.push(self.tasks.get(&target).unwrap());
        });

        tasks
    }
}
