use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, Operation};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_cache_item::cache_item;
use moon_common::path::{WorkspaceRelativePath, encode_component};
use moon_common::{Id, color, is_ci};
use moon_env_var::GlobalEnvBag;
use moon_platform::{BoxedPlatform, PlatformManager, Runtime};
use moon_project::Project;
use moon_time::to_millis;
use moon_workspace_graph::WorkspaceGraph;
use proto_core::UnresolvedVersionSpec;
use starbase_utils::fs;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use tracing::{debug, instrument};

cache_item!(
    pub struct DependenciesCacheState {
        pub last_hash: String,
        pub last_install_time: u128,
        pub last_tool_version: Option<UnresolvedVersionSpec>,
    }
);

#[instrument(skip_all)]
pub async fn install_deps(
    action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    runtime: &Runtime,
    project: Option<&Project>,
    packages_root: Option<&WorkspaceRelativePath>,
) -> miette::Result<ActionStatus> {
    if runtime.is_system() {
        return Ok(ActionStatus::Skipped);
    }

    let bag = GlobalEnvBag::instance();
    let pid = process::id().to_string();
    let log_label = runtime.label();
    let action_key = get_skip_key(runtime, project);

    let _lock = app_context
        .cache_engine
        .create_lock(format!("installDeps-{action_key}"))?;

    if let Some(value) = should_skip_action_matching("MOON_SKIP_INSTALL_DEPS", action_key) {
        debug!(
            env = value,
            "Skipping {} dependency install because {} is set",
            log_label,
            color::symbol("MOON_SKIP_INSTALL_DEPS")
        );

        return Ok(ActionStatus::Skipped);
    }

    if proto_core::is_offline() {
        debug!("No internet connection, skipping dependency install");

        return Ok(ActionStatus::Skipped);
    }

    if bag
        .get("INTERNAL_MOON_INSTALLING_DEPS")
        .is_some_and(|other_pid| other_pid != pid)
    {
        debug!("Detected another dependency install running, skipping dependency install");

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

    let registry = PlatformManager::read();
    let platform = registry.get_by_toolchain(&runtime.toolchain)?;

    let Some((lockfile_name, manifest_name)) = platform.get_dependency_configs()? else {
        debug!("No dependency manager configured for language, skipping dependency install");

        return Ok(ActionStatus::Skipped);
    };

    // Hash dependencies from all applicable manifests
    let manifests_hash = hash_manifests(
        action,
        &action_context,
        &app_context,
        &workspace_graph,
        project,
        platform,
        &manifest_name,
    )
    .await?;

    // Extract lockfile timestamp and tool version
    let mut lockfile_timestamp = track_lockfile(&app_context, project, &lockfile_name)?;
    let tool_version = runtime.requirement.to_spec();

    // Only install deps if a cache miss
    let mut state = app_context
        .cache_engine
        .state
        .load_state::<DependenciesCacheState>(get_state_path(&app_context, runtime, project))?;

    if
    // Lockfile doesn't exist
    lockfile_timestamp == 0
        // Dependencies haven't been installed yet
        || state.data.last_install_time == 0
        || !has_vendor_dir(&app_context, &runtime.toolchain, project)
        // Dependencies have changed since last run
        || state.data.last_install_time != lockfile_timestamp
        || manifests_hash
            .as_ref()
            .is_some_and(|hash| hash != &state.data.last_hash)
        // Toolchain version has changed
        || state.data.last_tool_version != tool_version
    {
        let working_dir = match project {
            Some(proj) => proj.root.clone(),
            None => match packages_root {
                Some(pr) => pr.to_logical_path(&app_context.workspace_root),
                None => app_context.workspace_root.clone(),
            },
        };

        // To avoid nested installs caused by child processes, we set this environment
        // variable with the current process ID and compare against it. If the IDs are
        // the same then multiple installs are happening in parallel in the same
        // process (via the pipeline), otherwise it's a child process.
        bag.set("INTERNAL_MOON_INSTALLING_DEPS", pid);

        debug!(
            "Installing {} dependencies in {}",
            log_label,
            color::path(&working_dir)
        );

        action.operations.extend(
            platform
                .install_deps(&action_context, runtime, &working_dir)
                .await?,
        );

        // Reload the timestamp as the lockfile may have changed from the install
        lockfile_timestamp = track_lockfile(&app_context, project, &lockfile_name)?;

        state.data.last_hash = manifests_hash.unwrap_or_default();
        state.data.last_install_time = lockfile_timestamp;
        state.data.last_tool_version = tool_version;
        state.save()?;

        return Ok(ActionStatus::Passed);
    }

    debug!("Lockfile or manifests have not changed since last run, skipping dependency install");

    Ok(ActionStatus::Skipped)
}

fn has_vendor_dir(app_context: &AppContext, toolchain: &Id, project: Option<&Project>) -> bool {
    let vendor_dir_name = match toolchain.as_str() {
        "bun" | "node" => "node_modules",
        // Ignore for other platforms
        _ => return true,
    };

    project
        .map(|proj| proj.root.join(vendor_dir_name))
        .unwrap_or_else(|| app_context.workspace_root.join(vendor_dir_name))
        .exists()
}

async fn hash_manifests(
    action: &mut Action,
    action_context: &ActionContext,
    app_context: &AppContext,
    workspace_graph: &WorkspaceGraph,
    project: Option<&Project>,
    platform: &BoxedPlatform,
    manifest_name: &str,
) -> miette::Result<Option<String>> {
    let mut operation = Operation::hash_generation();

    // If no manifests touched, then do nothing
    let has_touched_manifests = action_context
        .touched_files
        .iter()
        .any(|file| file.as_str().ends_with(manifest_name));

    if !has_touched_manifests {
        operation.finish(ActionStatus::Skipped);

        action.operations.push(operation);

        return Ok(None);
    }

    let mut hasher = app_context.cache_engine.hash.create_hasher(&action.label);

    // When running in the project root, include only their manifest
    if let Some(project) = project {
        let project_manifest = project.root.join(manifest_name);

        if project_manifest.exists() {
            platform
                .hash_manifest_deps(
                    &project_manifest,
                    &mut hasher,
                    &app_context.workspace_config.hasher,
                )
                .await?;
        }
    }
    // When running in the workspace root, include all project manifests
    else {
        for project in workspace_graph.projects.get_all_unexpanded() {
            let project_manifest = project.root.join(manifest_name);

            if project_manifest.exists() {
                platform
                    .hash_manifest_deps(
                        &project_manifest,
                        &mut hasher,
                        &app_context.workspace_config.hasher,
                    )
                    .await?;
            }
        }

        // And include the root manifest
        let root_manifest = app_context.workspace_root.join(manifest_name);

        if root_manifest.exists() {
            platform
                .hash_manifest_deps(
                    &root_manifest,
                    &mut hasher,
                    &app_context.workspace_config.hasher,
                )
                .await?;
        }
    }

    let hash = app_context.cache_engine.hash.save_manifest(&mut hasher)?;

    operation.meta.set_hash(&hash);
    operation.finish(ActionStatus::Passed);

    action.operations.push(operation);

    Ok(Some(hash))
}

fn track_lockfile(
    app_context: &AppContext,
    project: Option<&Project>,
    lockfile_name: &str,
) -> miette::Result<u128> {
    let lockfile_path = project
        .map(|proj| proj.root.join(lockfile_name))
        .unwrap_or_else(|| app_context.workspace_root.join(lockfile_name));
    let mut last_modified = 0;

    if lockfile_path.exists() {
        let meta = fs::metadata(&lockfile_path)?;

        if let Ok(timestamp) = meta.modified().or_else(|_| meta.created()) {
            last_modified = to_millis(timestamp);
        }
    }

    Ok(last_modified)
}

fn get_skip_key(runtime: &Runtime, project: Option<&Project>) -> String {
    format!(
        "{}:{}",
        runtime.id(),
        match project {
            Some(proj) => proj.id.as_str(),
            None => "*",
        }
    )
}

fn get_state_path(
    app_context: &AppContext,
    runtime: &Runtime,
    project: Option<&Project>,
) -> PathBuf {
    let state_path = PathBuf::from(format!(
        "installDeps-{}.json",
        encode_component(runtime.id())
    ));

    if let Some(project) = project {
        return app_context
            .cache_engine
            .state
            .get_project_dir(&project.id)
            .join(state_path);
    }

    state_path
}
