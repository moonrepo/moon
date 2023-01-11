use log::info;
use proto::{create_tool, enable_logging, ProtoError, ToolType};

pub async fn install(tool_type: ToolType, version: Option<String>) -> Result<(), ProtoError> {
    enable_logging();

    let version = version.unwrap_or_else(|| "latest".into());
    let mut tool = create_tool(&tool_type)?;

    info!(target: "proto:install", "Installing {} with version \"{}\"", tool.get_name(), version);

    if !tool.is_setup(&version).await? {
        tool.setup(&version).await?;
    }

    info!(target: "proto:install", "{} has been installed!", tool.get_name());

    Ok(())
}
