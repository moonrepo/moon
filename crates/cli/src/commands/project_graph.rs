use crate::helpers::load_workspace;
use moon_project_graph::project_graph::ProjectGraph;

pub async fn project_graph(project_id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let project_graph = ProjectGraph::generate(&workspace).await?;

    if let Some(id) = project_id {
        project_graph.load(id)?;
    } else {
        project_graph.load_all()?;
    }

    println!("{}", project_graph.to_dot());

    Ok(())
}
