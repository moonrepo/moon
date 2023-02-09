use crate::{
    config::{Config, CONFIG_NAME},
    helpers::enable_logging,
};
use log::{info, trace};
use proto::{color, create_tool, ProtoError, ToolType};
use std::{env, path::PathBuf};

pub async fn local(tool_type: ToolType, version: String) -> Result<(), ProtoError> {
    enable_logging();

    let mut tool = create_tool(&tool_type)?;

    tool.resolve_version(&version).await?;

    let local_path = env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(CONFIG_NAME);

    let mut config = if local_path.exists() {
        Config::load(&local_path)?
    } else {
        Config::default()
    };

    config
        .tools
        .insert(tool_type, tool.get_resolved_version().to_owned());

    config.save(&local_path)?;

    trace!(
        target: "proto:local",
        "Wrote the local version to {}",
        color::path(&local_path),
    );

    info!(
        target: "proto:local",
        "Set the local {} version to {}",
        tool.get_name(),
        tool.get_resolved_version(),
    );

    Ok(())
}
