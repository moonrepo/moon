use moon::{generate_project_graph, load_workspace_from_sandbox};
use moon_config::{GlobPath, HasherWalkStrategy, PartialHasherConfig, WorkspaceConfig};
use moon_runner::inputs_collector::collect_and_hash_inputs;
use moon_test_utils::{create_sandbox_with_config, get_cases_fixture_configs, Sandbox};
use moon_vcs::{BoxedVcs, Git};
use std::fs;
use std::path::Path;
use std::sync::Arc;

fn cases_sandbox() -> Sandbox {
    let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    create_sandbox_with_config(
        "cases",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    )
}

fn create_out_files(project_root: &Path) {
    let out_dir = project_root.join("out");

    fs::create_dir_all(&out_dir).unwrap();

    for i in 1..=5 {
        fs::write(out_dir.join(i.to_string()), i.to_string()).unwrap();
    }
}

fn load_vcs(workspace_root: &Path, workspace_config: &WorkspaceConfig) -> BoxedVcs {
    Box::new(
        Git::load(
            workspace_root,
            &workspace_config.vcs.default_branch,
            &workspace_config.vcs.remote_candidates,
        )
        .unwrap(),
    )
}

#[tokio::test]
async fn filters_using_input_globs() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let mut workspace = load_workspace_from_sandbox(sandbox.path()).await.unwrap();

    let project_graph = generate_project_graph(&mut workspace).await.unwrap();
    let vcs = load_vcs(&workspace.root, &workspace.config);

    let project = project_graph.get("outputsFiltering").unwrap();

    create_out_files(&project.root);

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutFile").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [
            ".moon/tasks.yml",
            ".moon/toolchain.yml",
            ".moon/workspace.yml",
            "outputs-filtering/out/1",
            "outputs-filtering/out/3",
            "outputs-filtering/out/5"
        ]
    );

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutDir").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    // .moon/*.yml files
    assert!(files.keys().collect::<Vec<_>>().len() == 3);

    // Out glob
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutGlob").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    // .moon/*.yml files
    assert!(files.keys().collect::<Vec<_>>().len() == 3);
}

#[tokio::test]
async fn filters_using_input_globs_in_glob_mode() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let mut workspace = load_workspace_from_sandbox(sandbox.path()).await.unwrap();

    let project_graph = generate_project_graph(&mut workspace).await.unwrap();
    let vcs = load_vcs(&workspace.root, &workspace.config);

    let mut workspace_config = Arc::into_inner(workspace.config).unwrap();
    workspace_config.hasher.walk_strategy = HasherWalkStrategy::Glob;
    workspace.config = Arc::new(workspace_config);

    let project = project_graph.get("outputsFiltering").unwrap();

    create_out_files(&project.root);

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutFile").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [
            ".moon/tasks.yml",
            ".moon/toolchain.yml",
            ".moon/workspace.yml",
            "outputs-filtering/out/1",
            "outputs-filtering/out/3",
            "outputs-filtering/out/5"
        ]
    );

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutDir").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    // .moon/*.yml files
    assert!(files.keys().collect::<Vec<_>>().len() == 3);

    // Out glob
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutGlob").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    // .moon/*.yml files
    assert!(files.keys().collect::<Vec<_>>().len() == 3);
}

#[tokio::test]
async fn filters_using_input_files() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let mut workspace = load_workspace_from_sandbox(sandbox.path()).await.unwrap();

    let project_graph = generate_project_graph(&mut workspace).await.unwrap();
    let vcs = load_vcs(&workspace.root, &workspace.config);

    let project = project_graph.get("outputsFiltering").unwrap();

    create_out_files(&project.root);

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutFile").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [
            ".moon/tasks.yml",
            ".moon/toolchain.yml",
            ".moon/workspace.yml",
            "outputs-filtering/out/1",
            "outputs-filtering/out/3"
        ]
    );

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutDir").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    // .moon/*.yml files
    assert!(files.keys().collect::<Vec<_>>().len() == 3);

    // Out glob
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutGlob").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    // .moon/*.yml files
    assert!(files.keys().collect::<Vec<_>>().len() == 3);
}

#[tokio::test]
async fn filters_using_input_files_in_glob_mode() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let mut workspace = load_workspace_from_sandbox(sandbox.path()).await.unwrap();

    let project_graph = generate_project_graph(&mut workspace).await.unwrap();
    let vcs = load_vcs(&workspace.root, &workspace.config);

    let mut workspace_config = Arc::into_inner(workspace.config).unwrap();
    workspace_config.hasher.walk_strategy = HasherWalkStrategy::Glob;
    workspace.config = Arc::new(workspace_config);

    let project = project_graph.get("outputsFiltering").unwrap();

    create_out_files(&project.root);

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutFile").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [
            ".moon/tasks.yml",
            ".moon/toolchain.yml",
            ".moon/workspace.yml",
            "outputs-filtering/out/1",
            "outputs-filtering/out/3"
        ]
    );

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutDir").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    // .moon/*.yml files
    assert!(files.keys().collect::<Vec<_>>().len() == 3);

    // Out glob
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutGlob").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    // .moon/*.yml files
    assert!(files.keys().collect::<Vec<_>>().len() == 3);
}

#[tokio::test]
async fn ignores_from_hasher_patterns() {
    let (mut workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    workspace_config.hasher = Some(PartialHasherConfig {
        ignore_patterns: Some(vec![GlobPath("**/out/*".into())]),
        ..Default::default()
    });

    let sandbox = create_sandbox_with_config(
        "cases",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    sandbox.enable_git();

    let mut workspace = load_workspace_from_sandbox(sandbox.path()).await.unwrap();

    let project_graph = generate_project_graph(&mut workspace).await.unwrap();
    let vcs = load_vcs(&workspace.root, &workspace.config);

    let project = project_graph.get("outputsFiltering").unwrap();

    create_out_files(&project.root);

    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutFile").unwrap(),
        &project.root,
        &workspace.root,
        &workspace.config.hasher,
    )
    .await
    .unwrap();

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [
            ".moon/tasks.yml",
            ".moon/toolchain.yml",
            ".moon/workspace.yml",
        ]
    );
}
