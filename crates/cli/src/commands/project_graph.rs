use crate::helpers::AnyError;
use moon::{generate_project_graph, load_workspace};

pub async fn project_graph(project_id: &Option<String>) -> Result<(), AnyError> {
    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;

    if let Some(id) = project_id {
        project_graph.get(id)?;
    } else {
        project_graph.get_all()?;
    }

    println!("{}", project_graph.to_dot());

    Ok(())
}
