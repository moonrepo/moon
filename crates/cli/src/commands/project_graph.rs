use monolith_workspace::Workspace;

pub async fn project_graph(workspace: &Workspace) -> Result<(), clap::Error> {
    let projects = &workspace.projects;

    // Preload all projects into the graph
    for id in projects.ids() {
        projects.get(id).unwrap();
    }

    println!("{:#?}", projects);

    Ok(())
}
