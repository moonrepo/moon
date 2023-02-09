use crate::helpers::detect_version_from_environment;
use proto::{create_tool, ProtoError, ToolType};

pub async fn bin(
    tool_type: ToolType,
    forced_version: Option<String>,
    shim: bool,
) -> Result<(), ProtoError> {
    let mut tool = create_tool(&tool_type)?;
    let version = detect_version_from_environment(&tool, &tool_type, forced_version).await?;

    tool.resolve_version(&version).await?;
    tool.find_bin_path().await?;

    if shim {
        tool.create_shims().await?;

        if let Some(shim_path) = tool.get_shim_path() {
            println!("{}", shim_path.to_string_lossy().to_string());

            return Ok(());
        }
    }

    println!("{}", tool.get_bin_path()?.to_string_lossy().to_string());

    Ok(())
}
