use crate::helpers::{create_progress_bar, AnyError};
use moon::{build_dep_graph, generate_project_graph, load_workspace};
use moon_runner::Runner;

pub async fn sync() -> Result<(), AnyError> {
    let done = create_progress_bar("Syncing projects...");

    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace)?;
    let mut project_count = 0;
    let mut dep_builder = build_dep_graph(&workspace, &project_graph);

    for project in project_graph.get_all()? {
        dep_builder.sync_project(project)?;
        project_count += 1;
    }

    let dep_graph = dep_builder.build();
    let mut runner = Runner::new(workspace);
    let results = runner.run(dep_graph, project_graph, None).await?;

    if runner.has_failed() {
        done("Failed to sync projects", false);
    } else {
        done(
            format!("Successfully synced {} projects", project_count).as_ref(),
            true,
        );
    }

    runner.render_results(&results)?;

    Ok(())
}
