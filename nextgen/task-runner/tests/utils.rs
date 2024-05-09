use moon_action::{ActionNode, RunTaskNode};
use moon_platform::Runtime;
use moon_task::Task;
use moon_workspace::Workspace;
use proto_core::ProtoEnvironment;
use std::path::Path;

pub fn create_workspace(root: &Path) -> Workspace {
    Workspace::load_from(root, ProtoEnvironment::new_testing(root)).unwrap()
}

pub fn create_node(task: &Task) -> ActionNode {
    ActionNode::RunTask(Box::new(RunTaskNode::new(
        task.target.clone(),
        Runtime::system(),
    )))
}
