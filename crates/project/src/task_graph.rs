use crate::errors::ProjectError;
use crate::project_graph::ProjectGraph;
use crate::task::Task;
use dep_graph::{DepGraph, Node};
use monolith_config::{ProjectID, Target};
use std::collections::HashMap;

fn split_target(target: &str) -> (ProjectID, String) {
    let split: Vec<&str> = target.split(':').collect();

    (String::from(split[0]), String::from(split[1]))
}

#[derive(Default)]
pub struct TaskGraph {
    nodes: HashMap<Target, Node<Target>>,

    tasks: HashMap<Target, Task>,
}

impl TaskGraph {
    pub fn new() -> Self {
        TaskGraph::default()
    }

    pub fn generate(
        &mut self,
        projects: &ProjectGraph,
        target: Target,
        parent_node: Option<&mut Node<Target>>,
    ) -> Result<(), ProjectError> {
        let (project_id, task_name) = split_target(&target);

        // Validate project first
        let project = projects.get(&project_id)?;

        // Validate task exists for project
        let task = match project.tasks.get(&task_name) {
            Some(t) => t,
            None => {
                return Err(ProjectError::UnconfiguredTask(task_name, project_id));
            }
        };

        // Add task to graph
        self.tasks.insert(target.clone(), task.clone());

        // Add dependencies
        let mut node = Node::new(target.clone());

        for dep_target in &task.deps {
            self.generate(projects, dep_target.clone(), Some(&mut node))?;
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
            .collect::<Vec<Node<Target>>>();
        let mut tasks: Vec<&Task> = vec![];

        DepGraph::new(&nodes).into_iter().for_each(|target| {
            tasks.push(self.tasks.get(&target).unwrap());
        });

        tasks
    }
}
