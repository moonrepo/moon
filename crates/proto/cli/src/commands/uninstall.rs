use log::info;
use proto::{create_tool, ProtoError, ToolType};

pub async fn uninstall(tool_type: ToolType, version: String) -> Result<(), ProtoError> {
    let mut tool = create_tool(&tool_type)?;

    info!(target: "proto:uninstall", "Uninstalling {:#?} with version \"{}\"", tool_type, version);

    if tool.is_setup(&version).await? {
        tool.teardown().await?;
    }

    info!(target: "proto:uninstall", "{:#?} has been uninstalled!", tool_type);

    Ok(())
}
