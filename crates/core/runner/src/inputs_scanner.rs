use crate::RunnerError;
use moon_hasher::convert_paths_to_strings;
use moon_project::Project;
use moon_task::Task;
use moon_utils::glob;
use moon_vcs::Vcs;
use rustc_hash::FxHashSet;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

type HashedInputs = BTreeMap<String, String>;

fn scan_with_globs(task: &Task, workspace_root: &Path) -> Result<Vec<PathBuf>, RunnerError> {
    let mut patterns = vec![];
    let workspace_root_str = workspace_root.to_string_lossy().to_string();

    // Find inputs
    for glob in &task.input_globs {
        patterns.push(glob.strip_prefix(&workspace_root_str).unwrap().to_owned());
    }

    // Exclude outputs
    for glob in &task.output_globs {
        patterns.push(format!(
            "!{}",
            glob.strip_prefix(&workspace_root_str).unwrap()
        ));
    }

    Ok(glob::walk(workspace_root, &patterns)?)
}

// Hash all inputs for a task, but exclude outputs
// and moon specific configuration files!
#[allow(clippy::borrowed_box)]
pub async fn scan_and_hash_inputs(
    vcs: &Box<dyn Vcs + Send + Sync>,
    project: &Project,
    task: &Task,
    workspace_root: &Path,
    use_globs: bool,
) -> Result<HashedInputs, RunnerError> {
    let mut files_to_hash = FxHashSet::default();
    let mut hashed_inputs: HashedInputs = BTreeMap::new();
    let globset = task.create_globset()?;

    // Gather inputs to hash
    if !task.input_paths.is_empty() {
        for input in &task.input_paths {
            if !task.output_paths.contains(input) {
                files_to_hash.insert(input.to_path_buf());
            }
        }
    }

    if !task.input_globs.is_empty() {
        // Walk the file system using globs to find inputs
        if use_globs {
            files_to_hash.extend(scan_with_globs(task, workspace_root)?);

            // Walk the file system using the VCS
        } else {
            let mut hashed_file_tree = vcs.get_file_tree_hashes(&project.source).await?;

            // Input globs are absolute paths, so we must do the same
            hashed_file_tree
                .retain(|f, _| globset.matches(workspace_root.join(f)).unwrap_or(false));

            hashed_inputs.extend(hashed_file_tree);
        }
    }

    let mut files_to_hash = convert_paths_to_strings(&files_to_hash, workspace_root)?;

    // Include local file changes so that development builds work.
    // Also run this LAST as it should take highest precedence!
    let local_files = vcs.get_touched_files().await?;

    if !local_files.all.is_empty() {
        // Only hash files that are within the task's inputs
        let files = local_files
            .all
            .into_iter()
            .filter(|f| globset.matches(workspace_root.join(f)).unwrap_or(false))
            .collect::<Vec<String>>();

        files_to_hash.extend(files);
    }

    // Hash all files that we've collected
    if !files_to_hash.is_empty() {
        hashed_inputs.extend(vcs.get_file_hashes(&files_to_hash, true).await?);
    }

    Ok(hashed_inputs)
}
