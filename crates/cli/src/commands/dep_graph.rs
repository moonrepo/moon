use crate::helpers::load_workspace;
use moon_project_graph::project_graph::ProjectGraph;
use moon_runner::DepGraph;
use moon_task::Target;

pub async fn dep_graph(target_id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let project_graph = ProjectGraph::generate(&workspace).await?;
    let mut graph = DepGraph::default();

    // Preload all projects
    project_graph.load_all()?;

    // Focus a target and its dependencies/dependents
    if let Some(id) = target_id {
        let target = Target::parse(id)?;

        graph.run_target(&target, &project_graph, &None)?;
        graph.run_target_dependents(&target, &project_graph)?;

    // Show all targets and actions
    } else {
        for project_id in project_graph.ids() {
            for task_id in project_graph.load(&project_id)?.tasks.keys() {
                graph.run_target(&Target::new(&project_id, task_id)?, &project_graph, &None)?;
            }
        }
    }

    println!("{}", graph.to_dot());

    Ok(())
}
