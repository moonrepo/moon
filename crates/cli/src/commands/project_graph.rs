use crate::helpers::load_workspace;

pub async fn project_graph(project_id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace = load_workspace().await?;
    let project_graph = workspace.generate_project_graph().await?;

    if let Some(id) = project_id {
        project_graph.get(id)?;
    } else {
        project_graph.get_all()?;
    }

    println!("{}", project_graph.to_dot());

    Ok(())
}
