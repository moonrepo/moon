mod task {
    use moon_common::path::RelativePathBuf;
    use moon_task::Task;
    use rustc_hash::FxHashSet;
    use starbase_sandbox::create_sandbox;

    #[tokio::test]
    async fn gets_all_input_files() {
        let sandbox = create_sandbox("files");

        let input_files = FxHashSet::from_iter([RelativePathBuf::from("c.jsx")]);
        let input_globs = FxHashSet::from_iter([RelativePathBuf::from("*.js")]);

        let task = Task {
            input_files,
            input_globs,
            ..Default::default()
        };

        let files = task.get_input_files(sandbox.path()).unwrap();

        assert!(files.len() == 3);
        assert!(files.contains(&RelativePathBuf::from("a.js")));
        assert!(files.contains(&RelativePathBuf::from("b.js")));
        assert!(files.contains(&RelativePathBuf::from("c.jsx")));
        assert!(!files.contains(&RelativePathBuf::from("d.rs")));
    }
}
