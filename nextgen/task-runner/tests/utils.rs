use moon_action::{ActionNode, RunTaskNode};
use moon_common::Id;
use moon_config::PlatformType;
use moon_platform::Runtime;
use moon_project::Project;
use moon_task::{Target, Task};
use moon_workspace::Workspace;
use proto_core::ProtoEnvironment;
use std::path::Path;

pub fn create_workspace(root: &Path) -> Workspace {
    Workspace::load_from(root, ProtoEnvironment::new_testing(root)).unwrap()
}

pub fn create_project(root: &Path) -> Project {
    Project {
        id: Id::raw("project"),
        root: root.join("apps/project"),
        source: "apps/project".into(),
        ..Default::default()
    }
}

pub fn create_task(project: &Project) -> Task {
    let mut task = Task {
        command: "build".into(),
        args: vec!["arg".into(), "--opt".into()],
        id: Id::raw("task"),
        target: Target::new(&project.id, "task").unwrap(),
        platform: PlatformType::System,
        ..Default::default()
    };
    task.env.insert("KEY".into(), "value".into());
    task
}

pub fn create_node(task: &Task) -> ActionNode {
    ActionNode::RunTask(Box::new(RunTaskNode::new(
        task.target.clone(),
        Runtime::system(),
    )))
}
