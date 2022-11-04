use moon_logger::trace;
use moon_workspace::Workspace;

use crate::helpers::{load_workspace, AnyError};

pub struct AppState {
    pub workspace: Workspace,
}

pub async fn init() -> Result<AppState, AnyError> {
    trace!("Creating state for application");
    let workspace = load_workspace().await?;
    workspace.projects.load_all()?;
    Ok(AppState { workspace })
}
