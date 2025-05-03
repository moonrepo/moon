use crate::operations::{ExecCommandOptions, exec_plugin_command, handle_on_exec};
use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, InstallDependenciesNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::{color, is_ci};
use moon_env_var::GlobalEnvBag;
use moon_pdk_api::InstallDependenciesInput;
use moon_workspace_graph::WorkspaceGraph;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, instrument};

// Temporarily match this with the legacy action!
use super::install_deps::DependenciesCacheState;

#[instrument(skip(action, action_context, app_context, workspace_graph))]
pub async fn install_dependencies(
    action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    node: &InstallDependenciesNode,
) -> miette::Result<ActionStatus> {
    let log_label = node.toolchain_id.as_str();
    let cache_engine = &app_context.cache_engine;

    // Create a file lock by toolchain ID, as this avoids colliding
    // setup's in this process and other parallel processes
    let _lock = cache_engine.create_lock(format!("installDependencies-{}", node.toolchain_id))?;

    // Skip this action if requested by the user
    if let Some(value) = should_skip_action_matching("MOON_SKIP_INSTALL_DEPS", &node.toolchain_id) {
        debug!(
            env = value,
            "Skipping {log_label} dependency install because {} is set",
            color::symbol("MOON_SKIP_INSTALL_DEPS")
        );

        return Ok(ActionStatus::Skipped);
    }

    // Installing dependencies requires an internet connection
    if proto_core::is_offline() {
        debug!("No internet connection, skipping dependency install");

        return Ok(ActionStatus::Skipped);
    }

    // When cache is write only, avoid install as user is typically force updating cache
    if app_context.cache_engine.is_write_only() {
        debug!("Force updating cache, skipping dependency install");

        return Ok(ActionStatus::Skipped);
    }

    // When running against affected files, avoid install as it interrupts the workflow,
    // especially when used with VSC hooks
    if action_context.affected.is_some() && !is_ci() {
        debug!("Running against affected files, skipping dependency install");

        return Ok(ActionStatus::Skipped);
    }

    // Load the toolchain and its state
    let toolchain = app_context
        .toolchain_registry
        .load(&node.toolchain_id)
        .await?;

    if !toolchain.has_func("install_dependencies").await {
        debug!("Skipping {log_label} dependency install as the toolchain does not support it");

        return Ok(ActionStatus::Skipped);
    }

    let deps_root = node.root.to_logical_path(&app_context.workspace_root);

    // When in CI, we can avoid installing dependencies if the vendor directory exists
    // because we can assume they've already been installed before moon runs!
    if is_ci()
        && has_vendor_installed_dependencies(
            &deps_root,
            toolchain.metadata.vendor_dir_name.as_deref(),
        )
    {
        debug!("In a CI environment and dependencies already exist, skipping dependency install");

        return Ok(ActionStatus::Skipped);
    }

    // Build input params
    let mut input = InstallDependenciesInput {
        context: app_context.toolchain_registry.create_context(),
        project: None,
        root: toolchain.to_virtual_path(&deps_root),
        toolchain_config: app_context
            .toolchain_registry
            .create_config(&toolchain.id, &app_context.toolchain_config),
        ..Default::default()
    };

    if let Some(project_id) = &node.project_id {
        let project = workspace_graph.get_project(project_id)?;

        input.project = Some(project.to_fragment());
        input.toolchain_config = app_context.toolchain_registry.create_merged_config(
            &toolchain.id,
            &app_context.toolchain_config,
            &project.config,
        );
    }

    // Extract commands from output
    let output = toolchain.install_dependencies(input).await?;

    if output.install_command.is_none() && output.dedupe_command.is_none() {
        return Ok(ActionStatus::Skipped);
    }

    // TODO caching

    let console = app_context.console.clone();

    let options = ExecCommandOptions {
        prefix: "install-dependencies".into(),
        working_dir: deps_root,
        on_exec: Some(Arc::new(move |cmd, attempts| {
            handle_on_exec(&console, cmd, attempts)
        })),
    };

    let hide_output = GlobalEnvBag::instance()
        .get("MOON_TEST_HIDE_INSTALL_OUTPUT")
        .is_some();

    if let Some(mut install) = output.install_command {
        debug!("Installing {log_label} dependencies");

        install.cache = None; // Disable
        install.command.stream = !hide_output;
        action
            .operations
            .extend(exec_plugin_command(app_context.clone(), &install, &options).await?);
    }

    if !is_ci() {
        if let Some(mut dedupe) = output.dedupe_command {
            debug!("Deduping {log_label} dependencies");

            dedupe.cache = None; // Disable
            dedupe.command.stream = !hide_output;
            action
                .operations
                .extend(exec_plugin_command(app_context, &dedupe, &options).await?);
        }
    }

    Ok(ActionStatus::Passed)
}

fn has_vendor_installed_dependencies(deps_root: &Path, vendor_dir_name: Option<&str>) -> bool {
    let Some(vendor_dir_name) = vendor_dir_name else {
        return false;
    };

    let vendor_dir = deps_root.join(vendor_dir_name);

    if !vendor_dir.exists() {
        return false;
    }

    match std::fs::read_dir(vendor_dir) {
        Ok(mut contents) => contents.next().is_some(),
        Err(_) => false,
    }
}
