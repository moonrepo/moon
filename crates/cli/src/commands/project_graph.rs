use crate::helpers::AnyError;
use moon::{build_project_graph, load_workspace};

pub async fn project_graph(project_id: &Option<String>) -> Result<(), AnyError> {
    let mut workspace = load_workspace().await?;
    let mut project_build = build_project_graph(&mut workspace).await?;

    if let Some(id) = project_id {
        project_build.load(id)?;
    } else {
        project_build.load_all()?;
    }

    let project_graph = project_build.build();

    println!("{}", project_graph.to_dot());

    Ok(())
}
