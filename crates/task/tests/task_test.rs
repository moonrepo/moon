use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::Output;
use moon_task::{Task, TaskFileInput, TaskFileOutput, TaskGlobInput, TaskGlobOutput};
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

    mod has_outputs {
        use super::*;

        #[test]
        fn false_when_no_outputs() {
            let task = Task::default();

            assert!(!task.has_outputs());
        }

        // The raw `outputs` config is populated at build time, while
        // `output_files`/`output_globs` only exist after expansion. A task
        // depending on a build task is resolved before expansion, so
        // `has_outputs` must account for the unexpanded form.
        #[test]
        fn true_when_only_unexpanded_outputs() {
            let task = Task {
                outputs: vec![Output::parse("build").unwrap()],
                ..Default::default()
            };

            assert!(task.has_outputs());
        }

        #[test]
        fn true_when_expanded_output_files() {
            let task = Task {
                output_files: FxHashMap::from_iter([(
                    WorkspaceRelativePathBuf::from("build"),
                    TaskFileOutput::default(),
                )]),
                ..Default::default()
            };

            assert!(task.has_outputs());
        }

        #[test]
        fn true_when_expanded_output_globs() {
            let task = Task {
                output_globs: FxHashMap::from_iter([(
                    WorkspaceRelativePathBuf::from("build/*"),
                    TaskGlobOutput::default(),
                )]),
                ..Default::default()
            };

            assert!(task.has_outputs());
        }
    }
}
