use moon_common::path::WorkspaceRelativePathBuf;
use moon_task::Task;
use rustc_hash::FxHashSet;
use starbase_sandbox::create_sandbox;

mod task {
    use super::*;

    #[test]
    fn gets_all_input_files() {
        let sandbox = create_sandbox("files");

        let task = Task {
            input_files: FxHashSet::from_iter([WorkspaceRelativePathBuf::from("c.jsx")]),
            input_globs: FxHashSet::from_iter([WorkspaceRelativePathBuf::from("*.js")]),
            ..Default::default()
        };

        let files = task.get_input_files(sandbox.path()).unwrap();

        assert_eq!(files.len(), 3);
        assert!(files.contains(&WorkspaceRelativePathBuf::from("a.js")));
        assert!(files.contains(&WorkspaceRelativePathBuf::from("b.js")));
        assert!(files.contains(&WorkspaceRelativePathBuf::from("c.jsx")));
        assert!(!files.contains(&WorkspaceRelativePathBuf::from("d.rs")));
    }

    #[test]
    fn filters_out_nonexistent_files() {
        let sandbox = create_sandbox("files");

        let task = Task {
            input_files: FxHashSet::from_iter([WorkspaceRelativePathBuf::from("nonexistent.jsx")]),
            input_globs: FxHashSet::from_iter([WorkspaceRelativePathBuf::from("*.py")]),
            ..Default::default()
        };

        let files = task.get_input_files(sandbox.path()).unwrap();

        assert_eq!(files.len(), 0);
    }
}
