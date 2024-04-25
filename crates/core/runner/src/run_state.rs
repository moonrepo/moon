use moon_cache_item::{cache_item, get_cache_mode};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_logger::{map_list, warn};
use starbase_archive::tar::{TarPacker, TarUnpacker};
use starbase_archive::Archiver;
use starbase_styles::color;
use starbase_utils::{fs, glob};
use std::path::{Path, PathBuf};

cache_item!(
    pub struct RunTargetState {
        pub exit_code: i32,
        pub hash: String,
        pub last_run_time: u128,
        pub target: String,
    }
);

fn create_archive<'o>(
    workspace_root: &'o Path,
    archive_file: &'o Path,
    output_paths: &[WorkspaceRelativePathBuf],
) -> Archiver<'o> {
    let mut archive = Archiver::new(workspace_root, archive_file);

    archive
}

pub fn archive_outputs(
    state_dir: &Path,
    archive_file: &Path,
    workspace_root: &Path,
    output_paths: &[WorkspaceRelativePathBuf],
) -> miette::Result<bool> {
    Ok(false)
}

pub fn hydrate_outputs(
    state_dir: &Path,
    archive_file: &Path,
    workspace_root: &Path,
    output_paths: &[WorkspaceRelativePathBuf],
) -> miette::Result<bool> {
    Ok(false)
}

pub fn get_output_logs(state_dir: &Path) -> (PathBuf, PathBuf) {
    (state_dir.join("stdout.log"), state_dir.join("stderr.log"))
}

/// Load the stdout.log and stderr.log files from the cache directory.
pub fn load_output_logs(state_dir: &Path) -> miette::Result<(String, String)> {
    let (stdout_path, stderr_path) = get_output_logs(state_dir);

    let stdout = if stdout_path.exists() {
        fs::read_file(stdout_path)?
    } else {
        String::new()
    };

    let stderr = if stderr_path.exists() {
        fs::read_file(stderr_path)?
    } else {
        String::new()
    };

    Ok((stdout, stderr))
}

/// Write stdout and stderr log files to the cache directory.
pub fn save_output_logs(state_dir: &Path, stdout: String, stderr: String) -> miette::Result<()> {
    let (stdout_path, stderr_path) = get_output_logs(state_dir);

    fs::write_file(stdout_path, stdout)?;
    fs::write_file(stderr_path, stderr)?;

    Ok(())
}
