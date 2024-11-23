use moon_action::Operation;
use moon_cache_item::cache_item;
use starbase_utils::fs;
use std::path::PathBuf;

cache_item!(
    pub struct TaskRunCacheState {
        pub exit_code: i32,
        pub hash: String,
        pub last_run_time: u128,
        pub target: String,
    }
);

pub fn read_stdlog_state_files(
    state_dir: PathBuf,
    operation: &mut Operation,
) -> miette::Result<()> {
    if let Some(output) = operation.get_output_mut() {
        let err_path = state_dir.join("stderr.log");
        let out_path = state_dir.join("stdout.log");

        if err_path.exists() {
            output.set_stderr(fs::read_file(err_path)?);
        }

        if out_path.exists() {
            output.set_stdout(fs::read_file(out_path)?);
        }
    }

    Ok(())
}

pub fn write_stdlog_state_files(state_dir: PathBuf, operation: &Operation) -> miette::Result<()> {
    let err_path = state_dir.join("stderr.log");
    let out_path = state_dir.join("stdout.log");

    if let Some(output) = operation.get_output() {
        fs::write_file(
            err_path,
            output
                .stderr
                .as_ref()
                .map(|log| log.as_bytes())
                .unwrap_or_default(),
        )?;

        fs::write_file(
            out_path,
            output
                .stdout
                .as_ref()
                .map(|log| log.as_bytes())
                .unwrap_or_default(),
        )?;
    } else {
        // Ensure logs from a previous run are removed
        fs::remove_file(err_path)?;
        fs::remove_file(out_path)?;
    }

    Ok(())
}
