use crate::errors::WorkspaceError;
use crate::Workspace;
use moon_config::TargetID;

#[allow(dead_code)]
pub async fn run_shell_target(
    _workspace: &Workspace,
    _target: TargetID,
) -> Result<(), WorkspaceError> {
    // TODO
    Ok(())
}
