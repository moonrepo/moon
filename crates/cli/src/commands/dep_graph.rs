use crate::helpers::{build_dep_graph, generate_project_graph, load_workspace};
use moon_task::Target;

pub async fn dep_graph(target_id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;
    let mut dep_builder = build_dep_graph(&workspace, &project_graph);

    // Focus a target and its dependencies/dependents
    if let Some(id) = target_id {
        let target = Target::parse(id)?;

        dep_builder.run_target(&target, None)?;
        dep_builder.run_dependents_for_target(&target)?;

    // Show all targets and actions
    } else {
        for project in project_graph.get_all()? {
            for task in project.tasks.values() {
                dep_builder.run_target(&task.target, None)?;
            }
        }
    }

    println!("{}", dep_builder.build().to_dot());

    Ok(())
}
