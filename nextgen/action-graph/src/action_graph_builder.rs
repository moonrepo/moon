use crate::action_node::ActionNode;
use moon_common::color;
use moon_platform::{PlatformManager, Runtime};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_task::Task;
use petgraph::prelude::*;
use rustc_hash::FxHashMap;
use tracing::debug;

pub struct ActionGraphBuilder<'app> {
    graph: StableGraph<ActionNode, ()>,
    indices: FxHashMap<ActionNode, NodeIndex>,
    project_graph: &'app ProjectGraph,
}

impl<'app> ActionGraphBuilder<'app> {
    pub fn get_index_from_node(&self, node: &ActionNode) -> Option<&NodeIndex> {
        self.indices.get(node)
    }

    pub fn get_target_runtime(&mut self, project: &Project, task: Option<&Task>) -> Runtime {
        if let Some(platform) = PlatformManager::read().find(|p| match task {
            Some(task) => p.matches(&task.platform, None),
            None => p.matches(&project.language.clone().into(), None),
        }) {
            return platform.get_runtime_from_config(Some(&project.config));
        }

        Runtime::system()
    }

    // ACTIONS

    pub fn setup_tool(&mut self, runtime: &Runtime) -> NodeIndex {
        let node = ActionNode::SetupTool {
            runtime: runtime.to_owned(),
        };

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        let sync_workspace_index = self.sync_workspace();
        let index = self.insert_node(node);

        self.graph.add_edge(index, sync_workspace_index, ());

        index
    }

    pub fn sync_project(&mut self, project: &Project) -> miette::Result<NodeIndex> {
        let runtime = self.get_target_runtime(project, None);

        let node = ActionNode::SyncProject {
            project: project.id.clone(),
            runtime: runtime.clone(),
        };

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(*index);
        }

        // Syncing depends on the language's tool to be installed
        let sync_workspace_index = self.sync_workspace();
        let setup_tool_index = self.setup_tool(&runtime);
        let index = self.insert_node(node);

        self.graph.add_edge(index, sync_workspace_index, ());
        self.graph.add_edge(index, setup_tool_index, ());

        // And we should also depend on other projects
        for dep_project_id in self.project_graph.dependencies_of(project)? {
            let dep_project = self.project_graph.get(dep_project_id)?;
            let dep_project_index = self.sync_project(&dep_project)?;

            if index != dep_project_index {
                self.graph.add_edge(index, dep_project_index, ());
            }
        }

        Ok(index)
    }

    pub fn sync_workspace(&mut self) -> NodeIndex {
        let node = ActionNode::SyncWorkspace;

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        self.insert_node(node)
    }

    // PRIVATE

    fn insert_node(&mut self, node: ActionNode) -> NodeIndex {
        debug!("Adding {} to graph", color::muted_light(node.label()));

        let index = self.graph.add_node(node.clone());

        self.indices.insert(node, index);

        index
    }
}
