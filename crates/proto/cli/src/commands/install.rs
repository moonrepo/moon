use log::info;
use proto::{create_tool, ProtoError, ToolType};

pub async fn install(tool_type: ToolType, version: Option<String>) -> Result<(), ProtoError> {
    let version = version.unwrap_or_else(|| "latest".into());
    let mut tool = create_tool(&tool_type)?;

    info!(target: "proto:install", "Installing {:#?} with version \"{}\"", tool_type, version);

    if !tool.is_setup(&version).await? {
        tool.setup(&version).await?;
    }

    info!(target: "proto:install", "{:#?} has been installed!", tool_type);

    Ok(())
}
