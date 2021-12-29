use moon_workspace::Workspace;

pub async fn setup(workspace: Workspace) -> Result<(), clap::Error> {
    workspace.toolchain.setup().await.unwrap(); // TODO error

    Ok(())
}
