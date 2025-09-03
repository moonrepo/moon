use crate::plugins::*;
use crate::utils::{create_hash_and_return_lock_if_changed, should_skip_action_matching};
use futures::StreamExt;
use futures::stream::FuturesOrdered;
use miette::IntoDiagnostic;
use moon_action::{Action, ActionStatus, InstallDependenciesNode, Operation};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::path::PathExt;
use moon_common::{color, is_ci, path::WorkspaceRelativePathBuf};
use moon_env_var::GlobalEnvBag;
use moon_feature_flags::glob_walk_with_options;
use moon_hash::hash_content;
use moon_pdk_api::{InstallDependenciesInput, ManifestDependency, ParseManifestInput};
use moon_project::ProjectFragment;
use moon_time::to_millis;
use moon_toolchain_plugin::ToolchainPlugin;
use moon_workspace_graph::WorkspaceGraph;
use starbase_utils::glob::GlobWalkOptions;
use starbase_utils::{fs, json::JsonValue};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

hash_content!(
    struct InstallDependenciesHash<'action> {
        action_node: &'action InstallDependenciesNode,
        lockfile_timestamp: Option<u128>,
        manifest_dependencies: BTreeMap<String, BTreeSet<String>>,
        manifest_paths: BTreeSet<WorkspaceRelativePathBuf>,
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
        warn!(
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

    let project = match &node.project_id {
        Some(project_id) => {
            let project = workspace_graph.get_project(project_id)?;

            input.project = Some(project.to_fragment());
            input.toolchain_config = app_context.toolchain_registry.create_merged_config(
                &toolchain.id,
                &app_context.toolchain_config,
                &project.config,
            );

            Some(project)
        }
        None => None,
    };

    // Create a lock if we haven't run before
    let Some(_lock) = create_hash_and_return_lock_if_changed(
        action,
        &app_context,
        create_hash_content(
            &app_context,
            &action_context,
            &toolchain,
            &deps_root,
            node,
            &input,
        )
        .await?,
    )
    .await?
    else {
        debug!(
            toolchain_id = toolchain.id.as_str(),
            "No {} toolchain changes since last run, skipping install", toolchain.metadata.name
        );

        return Ok(ActionStatus::Skipped);
    };

    // Extract from output
    let setup_op = Operation::setup_operation(action.get_prefix())?;
    let output = toolchain.install_dependencies(input).await?;
    let skipped = output.install_command.is_none()
        && output.dedupe_command.is_none()
        && output.operations.is_empty();

    // Execute commands
    let console = app_context.console.clone();

    let options = ExecCommandOptions {
        project,
        prefix: action.get_prefix().into(),
        working_dir: Some(deps_root),
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

    if !is_ci()
        && let Some(mut dedupe) = output.dedupe_command
    {
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

    finalize_action_operations(action, &toolchain, setup_op, output.operations, vec![])?;

    Ok(if skipped {
        ActionStatus::Skipped
    } else {
        ActionStatus::Passed
    })
}

async fn create_hash_content<'action>(
    app_context: &Arc<AppContext>,
    action_context: &Arc<ActionContext>,
    toolchain: &Arc<ToolchainPlugin>,
    deps_root: &Path,
    node: &'action InstallDependenciesNode,
    input: &'action InstallDependenciesInput,
) -> miette::Result<InstallDependenciesHash<'action>> {
    let mut content = InstallDependenciesHash {
        action_node: node,
        lockfile_timestamp: None,
        manifest_dependencies: BTreeMap::default(),
        manifest_paths: BTreeSet::default(),
        project: input.project.as_ref(),
        toolchain_config: &input.toolchain_config,
        vendor_dir_exists: false,
    };

    // Check if vendored already
    if let Some(vendor_dir_name) = &toolchain.metadata.vendor_dir_name {
        content.vendor_dir_exists = deps_root.join(vendor_dir_name).exists();
    }

    // Extract lockfile last modification
    for lock_file_name in &toolchain.metadata.lock_file_names {
        let lock_path = deps_root.join(lock_file_name);

        if lock_path.exists() {
            let meta = fs::metadata(&lock_path)?;

            if let Ok(timestamp) = meta.modified().or_else(|_| meta.created()) {
                content.lockfile_timestamp = Some(to_millis(timestamp));
                break;
            }
        }
    }

    // Extract dependencies from all applicable manifests
    for manifest_file_name in &toolchain.metadata.manifest_file_names {
        let has_touched_manifests = action_context
            .touched_files
            .iter()
            .any(|file| file.as_str().ends_with(manifest_file_name));

        // If no manifests touched, then do nothing and avoid all
        // this overhead! We can assume no dependencies have changed
        if has_touched_manifests {
            hash_manifest_contents(
                app_context,
                toolchain,
                deps_root,
                node,
                manifest_file_name,
                &mut content,
            )
            .await?;
        }
    }

    Ok(content)
}

async fn hash_manifest_contents<'action>(
    app_context: &Arc<AppContext>,
    toolchain: &Arc<ToolchainPlugin>,
    deps_root: &Path,
    node: &'action InstallDependenciesNode,
    manifest_file_name: &str,
    hash_content: &mut InstallDependenciesHash<'action>,
) -> miette::Result<()> {
    // Find all manifests in the workspace
    let deps_members = node.members.clone().unwrap_or_default();

    let mut manifest_paths =
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

    // Always include the root manifest
    if deps_root.join(manifest_file_name).exists() {
        manifest_paths.push(deps_root.join(manifest_file_name));
    }

    manifest_paths.sort();

    if manifest_paths.is_empty() {
        return Ok(());
    }

    // Parse each manifest concurrently
    let mut futures = FuturesOrdered::new();

    for manifest_path in manifest_paths {
        let app_context = Arc::clone(app_context);
        let toolchain = Arc::clone(toolchain);

        if let Ok(rel_path) = manifest_path.relative_to(&app_context.workspace_root) {
            hash_content.manifest_paths.insert(rel_path);
        }

        futures.push_back(tokio::spawn(async move {
            toolchain
                .parse_manifest(ParseManifestInput {
                    context: app_context.toolchain_registry.create_context(),
                    path: toolchain.to_virtual_path(manifest_path),
                })
                .await
        }));
    }

    // Inject the manifest deps into the hash
    let mut inject_deps = |deps: BTreeMap<String, ManifestDependency>| {
        for (name, dep) in deps {
            if let Some(version) = dep.get_version() {
                hash_content
                    .manifest_dependencies
                    .entry(name)
                    .or_default()
                    .insert(version.to_string());
            }
        }
    };

    while let Some(result) = futures.next().await {
        let output = result.into_diagnostic()??;

        inject_deps(output.dependencies);
        inject_deps(output.dev_dependencies);
        inject_deps(output.build_dependencies);
        inject_deps(output.peer_dependencies);
    }

    Ok(())
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
