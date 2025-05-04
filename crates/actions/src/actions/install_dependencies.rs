use crate::plugins::{ExecCommandOptions, exec_plugin_command, handle_on_exec};
use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, InstallDependenciesNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::{color, is_ci};
use moon_env_var::GlobalEnvBag;
use moon_feature_flags::glob_walk_with_options;
use moon_hash::hash_content;
use moon_pdk_api::InstallDependenciesInput;
use moon_project::ProjectFragment;
use moon_time::to_millis;
use moon_toolchain_plugin::ToolchainPlugin;
use moon_workspace_graph::WorkspaceGraph;
use starbase_utils::glob::GlobWalkOptions;
use starbase_utils::{fs, json::JsonValue};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, instrument};

hash_content!(
    struct InstallDependenciesHash<'action> {
        action_node: &'action InstallDependenciesNode,
        lockfile_timestamp: Option<u128>,
        project: Option<&'action ProjectFragment>,
        toolchain_config: &'action JsonValue,
        vendor_dir_exists: bool,
    }
);

#[instrument(skip(action, action_context, app_context, workspace_graph))]
pub async fn install_dependencies(
    action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    node: &InstallDependenciesNode,
) -> miette::Result<ActionStatus> {
    let deps_root = node.root.to_logical_path(&app_context.workspace_root);

    // Skip this action if requested by the user
    if let Some(value) = should_skip_action_matching("MOON_SKIP_INSTALL_DEPS", &node.toolchain_id) {
        debug!(
            root = node.root.as_str(),
            toolchain_id = node.toolchain_id.as_str(),
            env = value,
            "Skipping dependency install because {} is set",
            color::symbol("MOON_SKIP_INSTALL_DEPS")
        );

        return Ok(ActionStatus::Skipped);
    }

    // Installing dependencies requires an internet connection
    if proto_core::is_offline() {
        debug!(
            root = node.root.as_str(),
            toolchain_id = node.toolchain_id.as_str(),
            "No internet connection, skipping dependency install"
        );

        return Ok(ActionStatus::Skipped);
    }

    // When cache is write only, avoid install as user is typically force updating cache
    if app_context.cache_engine.is_write_only() {
        debug!(
            root = node.root.as_str(),
            toolchain_id = node.toolchain_id.as_str(),
            "Force updating cache, skipping dependency install"
        );

        return Ok(ActionStatus::Skipped);
    }

    // When running against affected files, avoid install as it interrupts the workflow,
    // especially when used with VSC hooks
    if action_context.affected.is_some() && !is_ci() {
        debug!(
            root = node.root.as_str(),
            toolchain_id = node.toolchain_id.as_str(),
            "Running against affected files, skipping dependency install"
        );

        return Ok(ActionStatus::Skipped);
    }

    // Load the toolchain and its state
    let toolchain = app_context
        .toolchain_registry
        .load(&node.toolchain_id)
        .await?;

    if !toolchain.has_func("install_dependencies").await {
        debug!(
            root = node.root.as_str(),
            toolchain_id = node.toolchain_id.as_str(),
            "Skipping dependency install as the toolchain does not support it"
        );

        return Ok(ActionStatus::Skipped);
    }

    // When in CI, we can avoid installing dependencies if the vendor directory exists
    // because we can assume they've already been installed before moon runs!
    if is_ci() && has_vendor_installed_dependencies(&toolchain, &deps_root) {
        debug!(
            root = node.root.as_str(),
            toolchain_id = node.toolchain_id.as_str(),
            "In a CI environment and dependencies already exist, skipping dependency install"
        );

        return Ok(ActionStatus::Skipped);
    }

    // Build the input
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

    // Create a lock if we haven't run before
    let Some(_lock) = app_context
        .cache_engine
        .create_hash_lock(
            action.get_prefix(),
            create_hash_content(&toolchain, &deps_root, node, &input)?,
        )
        .await?
    else {
        return Ok(ActionStatus::Skipped);
    };

    // Extract from output
    let output = toolchain.install_dependencies(input).await?;

    if output.install_command.is_none() && output.dedupe_command.is_none() {
        return Ok(ActionStatus::Skipped);
    }

    // TODO caching

    let console = app_context.console.clone();

    let options = ExecCommandOptions {
        prefix: action.get_prefix().into(),
        working_dir: deps_root,
        on_exec: Some(Arc::new(move |cmd, attempts| {
            handle_on_exec(&console, cmd, attempts)
        })),
    };

    let hide_output = GlobalEnvBag::instance()
        .get("MOON_TEST_HIDE_INSTALL_OUTPUT")
        .is_some();

    if let Some(mut install) = output.install_command {
        debug!(
            root = node.root.as_str(),
            toolchain_id = node.toolchain_id.as_str(),
            "Installing {} dependencies",
            toolchain.metadata.name
        );

        install.cache = None; // Disable
        install.command.stream = !hide_output;
        action
            .operations
            .extend(exec_plugin_command(app_context.clone(), &install, &options).await?);
    }

    if !is_ci() {
        if let Some(mut dedupe) = output.dedupe_command {
            debug!(
                root = node.root.as_str(),
                toolchain_id = node.toolchain_id.as_str(),
                "Deduping {} dependencies",
                toolchain.metadata.name
            );

            dedupe.cache = None; // Disable
            dedupe.command.stream = !hide_output;
            action
                .operations
                .extend(exec_plugin_command(app_context, &dedupe, &options).await?);
        }
    }

    Ok(ActionStatus::Passed)
}

fn create_hash_content<'action>(
    toolchain: &ToolchainPlugin,
    deps_root: &Path,
    node: &'action InstallDependenciesNode,
    input: &'action InstallDependenciesInput,
) -> miette::Result<InstallDependenciesHash<'action>> {
    let mut content = InstallDependenciesHash {
        action_node: node,
        lockfile_timestamp: None,
        project: input.project.as_ref(),
        toolchain_config: &input.toolchain_config,
        vendor_dir_exists: false,
    };

    // Extract lockfile last modification
    if let Some(lock_file_name) = &toolchain.metadata.lock_file_name {
        let lock_path = deps_root.join(lock_file_name);

        if lock_path.exists() {
            let meta = fs::metadata(&lock_path)?;

            if let Ok(timestamp) = meta.modified().or_else(|_| meta.created()) {
                content.lockfile_timestamp = Some(to_millis(timestamp));
            }
        }
    }

    // Extract dependencies from all applicable manifests
    if let Some(manifest_file_name) = &toolchain.metadata.manifest_file_name {
        let mut deps_members = node.members.clone().unwrap_or_default();
        deps_members.push(".".into());

        let _manifest_paths =
            glob_walk_with_options(deps_root, &deps_members, GlobWalkOptions::default().cache())?
                .into_iter()
                .filter_map(|path| {
                    if path.ends_with(manifest_file_name) {
                        Some(path)
                    } else if path.join(manifest_file_name).exists() {
                        Some(path.join(manifest_file_name))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

        // TODO
    }

    // Check if vendored already
    if let Some(vendor_dir_name) = &toolchain.metadata.vendor_dir_name {
        content.vendor_dir_exists = deps_root.join(vendor_dir_name).exists();
    }

    Ok(content)
}

fn has_vendor_installed_dependencies(toolchain: &ToolchainPlugin, deps_root: &Path) -> bool {
    let Some(vendor_dir_name) = &toolchain.metadata.vendor_dir_name else {
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
