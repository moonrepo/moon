use log::{debug, info};
use proto::{color, create_tool, enable_logging, ProtoError, ToolType};
use std::fs;

pub async fn list(tool_type: ToolType) -> Result<(), ProtoError> {
    enable_logging();

    let tool = create_tool(&tool_type)?;
    let install_dir = tool.get_install_dir()?;
    let tool_dir = install_dir.parent().unwrap(); // Without version

    debug!(target: "proto:list", "Finding versions in {}", color::path(tool_dir));

    info!(target: "proto:list", "Locally installed versions:");

    let handle_error = |e: std::io::Error| ProtoError::Fs(tool_dir.to_path_buf(), e.to_string());
    let mut install_count = 0;

    if tool_dir.exists() {
        for entry in fs::read_dir(tool_dir).map_err(handle_error)? {
            let entry = entry.map_err(handle_error)?;

            if entry.file_type().map_err(handle_error)?.is_dir() {
                install_count += 1;
                println!("{}", entry.file_name().to_string_lossy());
            }
        }
    }

    if install_count == 0 {
        eprintln!("No versions installed");
    }

    Ok(())
}
