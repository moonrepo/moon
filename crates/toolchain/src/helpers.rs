use crate::errors::ToolchainError;
use std::path::Path;
use tokio::process::Command;

pub async fn exec_command(bin: &Path, args: Vec<&str>, cwd: &Path) -> Result<(), ToolchainError> {
	let command_line = format!(
		"{} {}",
		bin.file_name().unwrap().to_str().unwrap(),
		args.join(" ")
	);

	let mut child = Command::new(bin.clone())
		.args(args)
		.current_dir(cwd)
		.spawn()
		.map_err(|_| ToolchainError::CommandFailed(command_line.clone()))?;

	child
		.wait()
		.await
		.map_err(|_| ToolchainError::CommandFailed(command_line.clone()))?;

	Ok(())
}
