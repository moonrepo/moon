//! Stage 1b: place the toolchains of the selected tasks, and the toolchains
//! that requests install dependencies for, into the layout graph, before
//! assembling, so that their dependencies roots are located concurrently,
//! instead of one at a time while the graph is assembled. Tasks are placed
//! even when their toolchains don't install dependencies, as their hashes
//! include the dependencies of their roots.

use super::select::{Event, ProvisionRequest, TaskPlan};
use crate::action_graph_options::ActionGraphBuilderOptions;
use moon_app_context::AppContext;
use moon_common::Id;
use moon_layout_graph::{LayoutGraph, LayoutGraphBuilder};
use moon_project::Project;
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::FxHashSet;
use std::sync::Arc;

/// Collect the pairs of a project's toolchains that install dependencies.
/// Tasks of the same project share most toolchains, so each pair is only
/// collected once, instead of once per task.
fn add_pairs<'plan>(
    pairs: &mut Vec<(Arc<Project>, Id)>,
    seen: &mut FxHashSet<(&'plan Id, &'plan Id)>,
    installs: &impl Fn(&Id) -> bool,
    project: &'plan Arc<Project>,
    toolchain_ids: &'plan [Id],
) {
    for toolchain_id in toolchain_ids {
        // Only mark a pair as seen once it's collected, as a request that
        // doesn't install may be followed by a task with the same pair
        if toolchain_id != "system"
            && installs(toolchain_id)
            && seen.insert((&project.id, toolchain_id))
        {
            pairs.push((Arc::clone(project), toolchain_id.to_owned()));
        }
    }
}

pub async fn place(
    app_context: &Arc<AppContext>,
    workspace_graph: &Arc<WorkspaceGraph>,
    options: &ActionGraphBuilderOptions,
    plan: &TaskPlan,
) -> miette::Result<LayoutGraph> {
    // Only toolchains that install dependencies resolve a root, otherwise
    // their install slot is the toolchain's setup
    let installs = |toolchain_id: &Id| {
        options.install_dependencies.is_enabled(toolchain_id)
            && app_context
                .toolchains_config
                .get_plugin_config(toolchain_id)
                .is_none_or(|config| config.install_dependencies)
    };

    let mut pairs = vec![];
    let mut seen = FxHashSet::default();
    let mut workspace_ids = vec![];

    for event in &plan.events {
        match event {
            Event::EnterTask { project, task, .. } => {
                add_pairs(&mut pairs, &mut seen, &|_| true, project, &task.toolchains);
            }
            Event::Request(_, request) => match request {
                ProvisionRequest::InstallDependencies(spec, project) => {
                    add_pairs(
                        &mut pairs,
                        &mut seen,
                        &installs,
                        project,
                        std::slice::from_ref(&spec.id),
                    );
                }
                ProvisionRequest::InstallDependenciesByProject(project) => {
                    add_pairs(
                        &mut pairs,
                        &mut seen,
                        &installs,
                        project,
                        &project.toolchains,
                    );
                }
                ProvisionRequest::InstallDependenciesRoot(spec) if installs(&spec.id) => {
                    workspace_ids.push(spec.id.clone());
                }
                ProvisionRequest::SetupEnvironmentRoot(spec)
                | ProvisionRequest::SetupToolchainAndEnvironment(spec)
                    if options.setup_environment.is_enabled(&spec.id) =>
                {
                    workspace_ids.push(spec.id.clone());
                }
                _ => {}
            },
            _ => {}
        };
    }

    let mut builder = LayoutGraphBuilder::new(Arc::clone(app_context), Arc::clone(workspace_graph));

    builder.place(pairs).await?;
    builder.place_workspace(workspace_ids).await?;

    Ok(builder.build())
}
