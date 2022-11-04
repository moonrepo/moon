use moon_logger::trace;
use moon_workspace::Workspace;

use crate::helpers::{load_workspace, AnyError};

pub async fn init() -> Result<Workspace, AnyError> {
    trace!("Creating state for application");
    let workspace = load_workspace().await?;
    workspace.projects.load_all()?;
    Ok(workspace)
}
