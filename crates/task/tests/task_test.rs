use moon_common::path::WorkspaceRelativePathBuf;
use moon_task::{Task, TaskFileInput, TaskGlobInput, TaskOptions};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::create_sandbox;

mod task {
    use super::*;

    #[test]
    fn gets_all_input_files() {
        let sandbox = create_sandbox("files");

        let task = Task {
            input_files: FxHashMap::from_iter([(
                WorkspaceRelativePathBuf::from("c.jsx"),
                TaskFileInput::default(),
            )]),
            input_globs: FxHashMap::from_iter([(
                WorkspaceRelativePathBuf::from("*.js"),
                TaskGlobInput::default(),
            )]),
            ..Default::default()
        };

        let files = task.get_input_files(sandbox.path()).unwrap();

        assert_eq!(files.len(), 3);
        assert!(files.contains(&sandbox.path().join("a.js")));
        assert!(files.contains(&sandbox.path().join("b.js")));
        assert!(files.contains(&sandbox.path().join("c.jsx")));
        assert!(!files.contains(&sandbox.path().join("d.rs")));
    }

    #[test]
    fn filters_out_nonexistent_files() {
        let sandbox = create_sandbox("files");

        let task = Task {
            input_files: FxHashMap::from_iter([(
                WorkspaceRelativePathBuf::from("nonexistent.jsx"),
                TaskFileInput::default(),
            )]),
            input_globs: FxHashMap::from_iter([(
                WorkspaceRelativePathBuf::from("*.py"),
                TaskGlobInput::default(),
            )]),
            ..Default::default()
        };

        let files = task.get_input_files(sandbox.path()).unwrap();

        assert_eq!(files.len(), 0);
    }

    #[test]
    fn filters_affected_files_by_project_boundary() {
        let sandbox = create_sandbox("files");
        let mut touched_files = FxHashSet::default();
        touched_files.insert(WorkspaceRelativePathBuf::from("project/a.js"));
        touched_files.insert(WorkspaceRelativePathBuf::from("shared/b.js"));

        let task = Task {
            input_globs: FxHashMap::from_iter([(
                WorkspaceRelativePathBuf::from("**/*.js"),
                TaskGlobInput::default(),
            )]),
            options: TaskOptions {
                affected_files_ignore_project_boundary: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let files = task
            .get_affected_files(sandbox.path(), &touched_files, "project")
            .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files.contains(&sandbox.path().join("project/a.js")));
        assert!(!files.contains(&sandbox.path().join("shared/b.js")));
    }

    #[test]
    fn includes_files_outside_project_when_boundary_ignored() {
        let sandbox = create_sandbox("files");
        let mut touched_files = FxHashSet::default();
        touched_files.insert(WorkspaceRelativePathBuf::from("project/a.js"));
        touched_files.insert(WorkspaceRelativePathBuf::from("shared/b.js"));

        let task = Task {
            input_globs: FxHashMap::from_iter([(
                WorkspaceRelativePathBuf::from("**/*.js"),
                TaskGlobInput::default(),
            )]),
            options: TaskOptions {
                affected_files_ignore_project_boundary: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let files = task
            .get_affected_files(sandbox.path(), &touched_files, "project")
            .unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.contains(&sandbox.path().join("project/a.js")));
        assert!(files.contains(&sandbox.path().join("shared/b.js")));
    }
}
