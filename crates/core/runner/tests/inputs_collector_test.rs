use moon::{generate_project_graph, load_workspace_from};
use moon_runner::inputs_collector::collect_and_hash_inputs;
use moon_test_utils::{create_sandbox_with_config, get_cases_fixture_configs, Sandbox};
use moon_vcs::VcsLoader;
use std::fs;
use std::path::Path;

fn cases_sandbox() -> Sandbox {
    let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    )
}

fn create_out_files(project_root: &Path) {
    let out_dir = project_root.join("out");

    fs::create_dir_all(&out_dir).unwrap();

    for i in 1..=5 {
        fs::write(out_dir.join(i.to_string()), i.to_string()).unwrap();
    }
}

#[tokio::test]
async fn filters_using_input_globs() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let project_graph = generate_project_graph(&mut workspace).await.unwrap();
    let vcs = VcsLoader::load(&workspace.root, &workspace.config).unwrap();

    let project = project_graph.get("outputsFiltering").unwrap();

    create_out_files(&project.root);

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutFile").unwrap(),
        &project.source,
        &workspace.root,
        false,
    )
    .await
    .unwrap();

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [
            "outputs-filtering/out/1",
            "outputs-filtering/out/3",
            "outputs-filtering/out/5"
        ]
    );

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutDir").unwrap(),
        &project.source,
        &workspace.root,
        false,
    )
    .await
    .unwrap();

    assert!(files.keys().collect::<Vec<_>>().is_empty());

    // Out glob
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutGlob").unwrap(),
        &project.source,
        &workspace.root,
        false,
    )
    .await
    .unwrap();

    assert!(files.keys().collect::<Vec<_>>().is_empty());
}

#[tokio::test]
async fn filters_using_input_globs_in_glob_mode() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let project_graph = generate_project_graph(&mut workspace).await.unwrap();
    let vcs = VcsLoader::load(&workspace.root, &workspace.config).unwrap();

    let project = project_graph.get("outputsFiltering").unwrap();

    create_out_files(&project.root);

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutFile").unwrap(),
        &project.source,
        &workspace.root,
        true,
    )
    .await
    .unwrap();

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [
            "outputs-filtering/out/1",
            "outputs-filtering/out/3",
            "outputs-filtering/out/5"
        ]
    );

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutDir").unwrap(),
        &project.source,
        &workspace.root,
        true,
    )
    .await
    .unwrap();

    assert!(files.keys().collect::<Vec<_>>().is_empty());

    // Out glob
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inGlobOutGlob").unwrap(),
        &project.source,
        &workspace.root,
        true,
    )
    .await
    .unwrap();

    assert!(files.keys().collect::<Vec<_>>().is_empty());
}

#[tokio::test]
async fn filters_using_input_files() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let project_graph = generate_project_graph(&mut workspace).await.unwrap();
    let vcs = VcsLoader::load(&workspace.root, &workspace.config).unwrap();

    let project = project_graph.get("outputsFiltering").unwrap();

    create_out_files(&project.root);

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutFile").unwrap(),
        &project.source,
        &workspace.root,
        false,
    )
    .await
    .unwrap();

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        ["outputs-filtering/out/1", "outputs-filtering/out/3"]
    );

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutDir").unwrap(),
        &project.source,
        &workspace.root,
        false,
    )
    .await
    .unwrap();

    assert!(files.keys().collect::<Vec<_>>().is_empty());

    // Out glob
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutGlob").unwrap(),
        &project.source,
        &workspace.root,
        false,
    )
    .await
    .unwrap();

    assert!(files.keys().collect::<Vec<_>>().is_empty());
}

#[tokio::test]
async fn filters_using_input_files_in_glob_mode() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let project_graph = generate_project_graph(&mut workspace).await.unwrap();
    let vcs = VcsLoader::load(&workspace.root, &workspace.config).unwrap();

    let project = project_graph.get("outputsFiltering").unwrap();

    create_out_files(&project.root);

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutFile").unwrap(),
        &project.source,
        &workspace.root,
        true,
    )
    .await
    .unwrap();

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        ["outputs-filtering/out/1", "outputs-filtering/out/3"]
    );

    // Out file
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutDir").unwrap(),
        &project.source,
        &workspace.root,
        true,
    )
    .await
    .unwrap();

    assert!(files.keys().collect::<Vec<_>>().is_empty());

    // Out glob
    let files = collect_and_hash_inputs(
        &vcs,
        project.get_task("inFileOutGlob").unwrap(),
        &project.source,
        &workspace.root,
        true,
    )
    .await
    .unwrap();

    assert!(files.keys().collect::<Vec<_>>().is_empty());
}
