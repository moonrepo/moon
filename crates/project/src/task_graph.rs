use crate::errors::ProjectError;
use crate::project_graph::ProjectGraph;
use crate::target::Target;
use crate::task::Task;
use crate::types::AffectedFiles;
use dep_graph::{DepGraph, Node};
use moon_config::TargetID;
use std::collections::HashMap;

#[derive(Default)]
pub struct TaskGraph {
    nodes: HashMap<TargetID, Node<TargetID>>,

    tasks: HashMap<TargetID, Task>,
}

impl TaskGraph {
    pub fn new(
        projects: &ProjectGraph,
        affected_files: &AffectedFiles,
        target: TargetID,
    ) -> Result<Self, ProjectError> {
        let mut graph = TaskGraph::default();
        graph.load(projects, affected_files, target, None)?;

        Ok(graph)
    }

    fn load(
        &mut self,
        projects: &ProjectGraph,
        affected_files: &AffectedFiles,
        target: TargetID,
        parent_node: Option<&mut Node<TargetID>>,
    ) -> Result<(), ProjectError> {
        let (project_id, task_id) = Target::parse(&target);

        // Validate project first
        let project = projects.get(&project_id)?;

        if !project.is_affected(affected_files) {
            return Ok(());
        }

        // Validate task exists for project
        let task = match project.tasks.get(&task_id) {
            Some(t) => t,
            None => {
                return Err(ProjectError::UnconfiguredTask(task_id, project_id));
            }
        };

        // Add task to graph
        self.tasks.insert(target.clone(), task.clone());

        // Add dependencies
        let mut node = Node::new(target.clone());

        for dep_target in &task.deps {
            self.load(
                projects,
                affected_files,
                dep_target.clone(),
                Some(&mut node),
            )?;
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
