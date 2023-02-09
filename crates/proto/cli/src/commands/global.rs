use crate::helpers::get_global_version_path;
use log::{info, trace};
use proto::{color, create_tool, ProtoError, ToolType};
use std::fs;

pub async fn global(tool_type: ToolType, version: String) -> Result<(), ProtoError> {
    let mut tool = create_tool(&tool_type)?;

    tool.resolve_version(&version).await?;

    let global_path = get_global_version_path(&tool)?;

    fs::write(&global_path, tool.get_resolved_version())
        .map_err(|e| ProtoError::Fs(global_path.to_path_buf(), e.to_string()))?;

    trace!(
        target: "proto:global",
        "Wrote the global version to {}",
        color::path(&global_path),
    );

    info!(
        target: "proto:global",
        "Set the global {} version to {}",
        tool.get_name(),
        tool.get_resolved_version(),
    );

    Ok(())
}
