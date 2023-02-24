use crate::RunnerError;
use moon_config::CONFIG_PROJECT_FILENAME;
use moon_hasher::convert_paths_to_strings;
use moon_task::Task;
use moon_utils::{glob, path};
use moon_vcs::Vcs;
use rustc_hash::FxHashSet;
use std::{collections::BTreeMap, path::Path};

type HashedInputs = BTreeMap<String, String>;

fn is_valid_input_source(
    task: &Task,
    input_globset: &glob::GlobSet,
    output_globset: &glob::GlobSet,
    workspace_root: &Path,
    workspace_relative_input: &str,
) -> bool {
    // Don't invalidate existing hashes when moon.yml changes
    // as we already hash the contents of each task!
    if workspace_relative_input.ends_with(CONFIG_PROJECT_FILENAME) {
        return false;
    }

    let absolute_input = workspace_root.join(workspace_relative_input);

    // Remove outputs first
    if output_globset.matches(workspace_relative_input) {
        return false;
    }

    for output in &task.output_paths {
        if &absolute_input == output || absolute_input.starts_with(output) {
            return false;
        }
    }

    // Filter inputs last
    task.input_paths.contains(&absolute_input) || input_globset.matches(workspace_relative_input)
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
    let input_globset = glob::GlobSet::new(&task.input_globs, &FxHashSet::default())?;
    let output_globset = glob::GlobSet::new(&task.output_globs, &FxHashSet::default())?;

    // 1: Collect inputs as a set of absolute paths

    if !task.input_paths.is_empty() {
        for input in &task.input_paths {
            files_to_hash.insert(input.to_path_buf());
        }
    }

    if !task.input_globs.is_empty() {
        // Collect inputs by walking and globbing the file system
        if use_globs {
            files_to_hash.extend(glob::walk(workspace_root, &task.input_globs)?);

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

    hashed_inputs.retain(|f, _| {
        is_valid_input_source(task, &input_globset, &output_globset, workspace_root, f)
    });

    // 4: Normalize input key paths

    hashed_inputs = hashed_inputs
        .into_iter()
        .map(|(k, v)| (path::standardize_separators(k), v))
        .collect::<BTreeMap<_, _>>();

    Ok(hashed_inputs)
}
