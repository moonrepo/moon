use crate::helpers::{create_progress_bar, AnyError};
use moon::{build_dep_graph, generate_project_graph, load_workspace};
use moon_action_pipeline::Pipeline;

pub async fn sync() -> Result<(), AnyError> {
    let done = create_progress_bar("Syncing projects...");

    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;
    let mut project_count = 0;
    let mut dep_builder = build_dep_graph(&workspace, &project_graph);

    for project in project_graph.get_all()? {
        dep_builder.sync_project(project)?;
        project_count += 1;
    }

    let dep_graph = dep_builder.build();

    let mut pipeline = Pipeline::new(workspace, project_graph);
    let results = pipeline.run(dep_graph, None).await?;

    done(
        format!("Successfully synced {project_count} projects"),
        true,
    );

    pipeline.render_results(&results)?;

    Ok(())
}
