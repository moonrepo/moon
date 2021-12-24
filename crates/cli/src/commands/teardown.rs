use monolith_workspace::Workspace;

pub async fn teardown(workspace: Workspace) -> Result<(), clap::Error> {
    workspace.toolchain.teardown().await.unwrap(); // TODO error

    Ok(())
}
