use moon_workspace::Workspace;

pub async fn ci() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    // Load all projects into the graph
    for pid in workspace.projects.ids() {
        workspace.projects.load(&pid)?;
    }

    let mut root_package = workspace.load_package_json().await?;

    workspace.toolchain.setup(&mut root_package).await?;

    Ok(())
}
