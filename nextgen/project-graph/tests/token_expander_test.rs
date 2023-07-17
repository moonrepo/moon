use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::TaskConfig;
use moon_project::Project;
use moon_project_graph2::{TokenExpander, TokenScope};
use moon_task::{Target, Task};
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;

fn create_project(workspace_root: &Path) -> Project {
    Project {
        id: Id::raw("project"),
        source: WorkspaceRelativePathBuf::from("source/path"),
        root: workspace_root.join("source/path"),
        // file_groups: create_file_groups("files-and-dirs"),
        ..Project::default()
    }
}

fn create_task(config: Option<TaskConfig>) -> Task {
    let mut task = Task {
        id: Id::raw("task"),
        target: Target::new("project", "task").unwrap(),
        ..Task::default()
    };

    if let Some(cfg) = config {
        task.inputs = cfg.inputs.unwrap_or_default();
        task.outputs = cfg.outputs.unwrap_or_default();
    }

    task
}

mod task_expander {
    use super::*;

    mod command {
        use super::*;

        #[test]
        #[should_panic(expected = "Token @files(sources) cannot be used within task commands.")]
        fn errors_for_func() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task(None);

            task.command = "@files(sources)".into();

            let expander = TokenExpander {
                scope: TokenScope::Command,
                project: &project,
                task: &task,
                workspace_root: sandbox.path(),
            };

            expander.expand_command().unwrap();
        }
    }
}
