use moon_workspace::Workspace;

pub async fn project_graph(id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    // Force load projects into the graph
    if let Some(pid) = id {
        workspace.projects.load(pid)?;
    } else {
        for pid in workspace.projects.ids() {
            workspace.projects.load(&pid)?;
        }
    }

    println!("{}", workspace.projects.to_dot());

    Ok(())
}
