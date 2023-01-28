use human_sort::compare;
use log::{debug, info};
use proto::{create_tool, enable_logging, ProtoError, ToolType};
use std::io::{self, Write};

// TODO: only show LTS, dont show pre-releases?
pub async fn list_remote(tool_type: ToolType) -> Result<(), ProtoError> {
    enable_logging();

    let tool = create_tool(&tool_type)?;

    debug!(target: "proto:list-remote", "Loading manifest");

    let manifest = tool.load_manifest().await?;

    info!(target: "proto:list-remote", "Available versions:");

    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(stdout);
    let mut releases = manifest.versions.values().collect::<Vec<_>>();

    releases.sort_by(|a, d| compare(&a.version, &d.version));

    for release in releases {
        writeln!(handle, "{}", release.version).unwrap();
    }

    Ok(())
}
