use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::{InputPath, LanguageType, OutputPath, ProjectType};
use moon_project::{FileGroup, Project};
use moon_task::{Target, Task};
use moon_task_expander::TasksExpander;
use starbase_sandbox::create_empty_sandbox;
use std::env;
use std::path::Path;

fn create_project(workspace_root: &Path) -> Project {
    let source = WorkspaceRelativePathBuf::from("project/source");

    Project {
        id: Id::raw("project"),
        root: workspace_root.join(source.as_str()),
        source,
        ..Default::default()
    }
}

fn create_task() -> Task {
    Task {
        id: Id::raw("task"),
        target: Target::new("project", "task").unwrap(),
        ..Default::default()
    }
}

mod tasks_expander {
    use super::*;

    mod command {
        use super::*;

        #[test]
        #[should_panic(expected = "Token @dirs(group) cannot be used within task commands.")]
        fn errors_on_token_funcs() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "@dirs(group)".into();

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_command(&mut task)
            .unwrap();
        }

        #[test]
        fn replaces_token_vars() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "./$project/bin".into();

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_command(&mut task)
            .unwrap();

            assert_eq!(task.command, "./project/bin");
        }

        #[test]
        fn replaces_env_vars() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "./$FOO/${BAR}/$BAZ_QUX".into();

            env::set_var("FOO", "foo");
            env::set_var("BAZ_QUX", "baz-qux");

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_command(&mut task)
            .unwrap();

            env::remove_var("FOO");
            env::remove_var("BAZ_QUX");

            assert_eq!(task.command, "./foo//baz-qux");
        }
    }
}
