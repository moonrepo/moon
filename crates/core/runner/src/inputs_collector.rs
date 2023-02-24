use crate::RunnerError;
use moon_config::CONFIG_PROJECT_FILENAME;
use moon_hasher::convert_paths_to_strings;
use moon_task::Task;
use moon_utils::{glob, path};
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
    task: &Task,
    project_source: &str,
    workspace_root: &Path,
    use_globs: bool,
) -> Result<HashedInputs, RunnerError> {
    let mut files_to_hash = FxHashSet::default(); // Absolute paths
    let mut hashed_inputs: HashedInputs = BTreeMap::new();
    let globset = task.create_globset()?;

    // 1: Collect inputs as a set of absolute paths

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
            hashed_inputs.extend(vcs.get_file_tree_hashes(project_source).await?);
        }
    }

    // Include local file changes so that development builds work.
    // Also run this LAST as it should take highest precedence!
    for local_file in vcs.get_touched_files().await?.all {
        files_to_hash.insert(workspace_root.join(local_file));
    }

    // 2: Convert to workspace relative paths and extract file hashes

    let files_to_hash = convert_paths_to_strings(&files_to_hash, workspace_root)?;

    if !files_to_hash.is_empty() {
        hashed_inputs.extend(vcs.get_file_hashes(&files_to_hash, true).await?);
    }

    // 3: Filter hashes to applicable inputs

    hashed_inputs.retain(|f, _| globset.matches(f));

    // 4: Remove outputs as sources

    // This is gross, a better way???
    let mut rel_output_paths = vec![];

    for output in &task.output_paths {
        rel_output_paths.push(path::to_string(
            output.strip_prefix(workspace_root).unwrap(),
        )?);
    }

    hashed_inputs.retain(|f, _| {
        // Don't invalidate existing hashes when moon.yml changes
        // as we already hash the contents of each task!
        if f.ends_with(CONFIG_PROJECT_FILENAME) {
            false
        } else {
            rel_output_paths.iter().all(|o| !f.starts_with(o))
        }
    });

    Ok(hashed_inputs)
}
