use crate::errors::ActionError;
use moon_lang_node::{package::PackageJson, tsconfig::TsConfigJson};
use moon_plugin_node::NodeTargetHasher;
use moon_project::Project;
use moon_task::{ExpandedFiles, Task};
use moon_utils::path::to_string;
use moon_workspace::Workspace;
use std::path::Path;

fn convert_paths_to_strings(
    paths: &ExpandedFiles,
    workspace_root: &Path,
) -> Result<Vec<String>, ActionError> {
    let mut files: Vec<String> = vec![];

    for path in paths {
        // Inputs may not exist and `git hash-object` will fail if you pass an unknown file
        if path.exists() {
            // We also need to use relative paths from the workspace root,
            // so that it works across machines
            let rel_path = if path.starts_with(workspace_root) {
                path.strip_prefix(workspace_root).unwrap()
            } else {
                path
            };

            files.push(to_string(rel_path)?);
        }
    }

    Ok(files)
}

pub async fn create_target_hasher(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
    passthrough_args: &[String],
) -> Result<NodeTargetHasher, ActionError> {
    let vcs = &workspace.vcs;
    let globset = task.create_globset()?;
    let mut hasher = NodeTargetHasher::new(workspace.config.node.version.clone());

    hasher.hash_project_deps(project.get_dependencies());
    hasher.hash_task(task);
    hasher.hash_args(passthrough_args);

    // Hash root configs first
    if let Some(root_package) = PackageJson::read(&workspace.root)? {
        hasher.hash_package_json(&root_package);
    }

    if let Some(root_tsconfig) = TsConfigJson::read_with_name(
        &workspace.root,
        &workspace.config.typescript.root_config_file_name,
    )? {
        hasher.hash_tsconfig_json(&root_tsconfig);
    }

    // Hash project configs second so they can override
    if let Some(package) = PackageJson::read(&project.root)? {
        hasher.hash_package_json(&package);
    }

    if let Some(tsconfig) = TsConfigJson::read_with_name(
        &project.root,
        &workspace.config.typescript.project_config_file_name,
    )? {
        hasher.hash_tsconfig_json(&tsconfig);
    }

    // For input files, hash them with the vcs layer first
    if !task.input_paths.is_empty() {
        let mut files = convert_paths_to_strings(&task.input_paths, &workspace.root)?;

        // Sort for deterministic caching within the vcs layer
        files.sort();

        if !files.is_empty() {
            hasher.hash_inputs(vcs.get_file_hashes(&files).await?);
        }
    }

    // For input globs, it's much more performant to:
    //  `git ls-tree` -> match against glob patterns
    // Then it is to:
    //  glob + walk the file system -> `git hash-object`
    if !task.input_globs.is_empty() {
        let mut hashed_file_tree = vcs.get_file_tree_hashes(&project.source).await?;

        // Input globs are absolute paths, so we must do the same
        hashed_file_tree.retain(|k, _| globset.matches(&workspace.root.join(k)).unwrap_or(false));

        hasher.hash_inputs(hashed_file_tree);
    }

    // Include local file changes so that development builds work.
    // Also run this LAST as it should take highest precedence!
    let local_files = vcs.get_touched_files().await?;

    if !local_files.all.is_empty() {
        // Only hash files that are within the task's inputs
        let mut files = local_files
            .all
            .into_iter()
            .filter(|f| {
                // Deleted files will crash `git hash-object`
                !local_files.deleted.contains(f)
                    && globset.matches(&workspace.root.join(f)).unwrap_or(false)
            })
            .collect::<Vec<String>>();

        // Sort for deterministic caching within the vcs layer
        files.sort();

        if !files.is_empty() {
            hasher.hash_inputs(vcs.get_file_hashes(&files).await?);
        }
    }

    Ok(hasher)
}
