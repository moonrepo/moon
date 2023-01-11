use proto::{create_tool, ProtoError, ToolType};

pub async fn install(tool: ToolType, version: Option<String>) -> Result<(), ProtoError> {
    let version = version.unwrap_or_else(|| "latest".into());
    let mut tool = create_tool(tool, &version)?;

    if !tool.is_setup(&version).await? {
        tool.setup(&version).await?;
    }

    Ok(())
}
