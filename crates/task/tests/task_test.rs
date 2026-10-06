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

    mod create_globset {
        use super::*;
        use std::sync::Arc;

        fn create_task(inputs: &[&str], outputs: &[&str]) -> Task {
            Task {
                input_globs: inputs
                    .iter()
                    .map(|glob| {
                        (
                            WorkspaceRelativePathBuf::from(*glob),
                            TaskGlobInput::default(),
                        )
                    })
                    .collect(),
                output_globs: outputs
                    .iter()
                    .map(|glob| {
                        (
                            WorkspaceRelativePathBuf::from(*glob),
                            TaskGlobOutput::default(),
                        )
                    })
                    .collect(),
                ..Default::default()
            }
        }

        #[test]
        fn shares_compiled_sets_with_the_same_patterns() {
            let a = create_task(&["src/**/*", "*.json"], &["dist/**/*"])
                .create_globset()
                .unwrap();
            let b = create_task(&["*.json", "src/**/*"], &["dist/**/*"])
                .create_globset()
                .unwrap();

            assert!(Arc::ptr_eq(&a, &b));
            assert!(a.matches("src/index.ts"));
            assert!(a.matches("package.json"));
            assert!(!a.matches("dist/index.js"));
            assert!(!a.matches("other/file.ts"));
        }

        #[test]
        fn doesnt_share_compiled_sets_with_different_patterns() {
            let a = create_task(&["src/**/*"], &[]).create_globset().unwrap();
            let b = create_task(&["src/**/*"], &["dist/**/*"])
                .create_globset()
                .unwrap();
            let c = create_task(&["lib/**/*"], &[]).create_globset().unwrap();

            assert!(!Arc::ptr_eq(&a, &b));
            assert!(!Arc::ptr_eq(&a, &c));
            assert!(!Arc::ptr_eq(&b, &c));
        }

        #[test]
        fn treats_negated_inputs_as_negations() {
            let globset = create_task(&["src/**/*", "!src/**/*.test.ts"], &[])
                .create_globset()
                .unwrap();

            assert!(globset.matches("src/index.ts"));
            assert!(!globset.matches("src/index.test.ts"));
        }
    }

    mod get_affected_files {
        use super::*;
        use rustc_hash::FxHashSet;

        fn create_changed_files(files: &[&str]) -> FxHashSet<WorkspaceRelativePathBuf> {
            files
                .iter()
                .map(|file| WorkspaceRelativePathBuf::from(*file))
                .collect()
        }

        #[test]
        fn matches_input_files() {
            let sandbox = create_sandbox("files");
            let root = sandbox.path();

            let task = Task {
                input_files: FxHashMap::from_iter([(
                    WorkspaceRelativePathBuf::from("c.jsx"),
                    TaskFileInput::default(),
                )]),
                ..Default::default()
            };

            let files = task
                .get_affected_files(root, &create_changed_files(&["a.js", "c.jsx"]))
                .unwrap();

            assert_eq!(files, vec![root.join("c.jsx")]);
        }

        #[test]
        fn matches_input_globs() {
            let sandbox = create_sandbox("files");
            let root = sandbox.path();

            let task = Task {
                input_globs: FxHashMap::from_iter([(
                    WorkspaceRelativePathBuf::from("*.js"),
                    TaskGlobInput::default(),
                )]),
                ..Default::default()
            };

            let mut files = task
                .get_affected_files(root, &create_changed_files(&["a.js", "b.js", "c.jsx"]))
                .unwrap();
            files.sort();

            assert_eq!(files, vec![root.join("a.js"), root.join("b.js")]);
        }

        #[test]
        fn matches_nothing_without_inputs() {
            let sandbox = create_sandbox("files");
            let root = sandbox.path();

            // Output globs only negate, they never match
            let task = Task {
                output_globs: FxHashMap::from_iter([(
                    WorkspaceRelativePathBuf::from("*.js"),
                    TaskGlobOutput::default(),
                )]),
                ..Default::default()
            };

            let files = task
                .get_affected_files(root, &create_changed_files(&["a.js", "c.jsx"]))
                .unwrap();

            assert!(files.is_empty());
        }
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
