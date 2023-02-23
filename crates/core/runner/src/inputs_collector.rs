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

fn collect_with_globs(task: &Task, workspace_root: &Path) -> Result<Vec<PathBuf>, RunnerError> {
    let mut patterns = vec![];

    // Find inputs
    for glob in &task.input_globs {
        patterns.push(glob.to_owned());
    }

    // Exclude outputs
    for glob in &task.output_globs {
        patterns.push(format!("!{glob}"));
    }

    Ok(glob::walk(workspace_root, &patterns)?)
}

// Hash all inputs for a task, but exclude outputs
// and moon specific configuration files!
#[allow(clippy::borrowed_box)]
pub async fn collect_and_hash_inputs(
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
            files_to_hash.insert(input.to_path_buf());
        }
    }

    if !task.input_globs.is_empty() {
        // Collect inputs by walking and globbing the file system
        if use_globs {
            files_to_hash.extend(collect_with_globs(task, workspace_root)?);

            // Collect inputs by querying VCS then matching against globs
        } else {
            let mut hashed_file_tree = vcs.get_file_tree_hashes(&project.source).await?;

            // Filter out non-matching inputs
            hashed_file_tree.retain(|f, _| globset.matches(f));

            hashed_inputs.extend(hashed_file_tree);
        }
    }

    // Convert absolute paths to workspace relative path strings
    let mut files_to_hash = convert_paths_to_strings(&files_to_hash, workspace_root)?;

    // Include local file changes so that development builds work.
    // Also run this LAST as it should take highest precedence!
    files_to_hash.extend(vcs.get_touched_files().await?.all);

    // Filter out inputs that overlap with outputs! This is very important!
    files_to_hash.retain(|f| globset.matches(f));

    // Hash all files that we've collected
    if !files_to_hash.is_empty() {
        hashed_inputs.extend(vcs.get_file_hashes(&files_to_hash, true).await?);
    }

    Ok(hashed_inputs)
}
