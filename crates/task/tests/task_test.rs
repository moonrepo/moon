use moon_common::path::WorkspaceRelativePathBuf;
use moon_task::{Task, TaskFileInput, TaskGlobInput};
use rustc_hash::FxHashMap;
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
}
