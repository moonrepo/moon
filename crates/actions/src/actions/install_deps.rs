use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, Operation};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_cache_item::cache_item;
use moon_common::color;
use moon_common::path::encode_component;
use moon_platform::{BoxedPlatform, PlatformManager, Runtime};
use moon_project::Project;
use moon_time::{now_millis, to_millis};
use starbase_utils::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, process};
use tracing::{debug, instrument};

cache_item!(
    pub struct DependenciesCacheState {
        pub last_hash: String,
        pub last_install_time: u128,
    }
);

#[instrument(skip_all)]
pub async fn install_deps(
    action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    runtime: &Runtime,
    project: Option<&Project>,
) -> miette::Result<ActionStatus> {
    if runtime.platform.is_system() {
        return Ok(ActionStatus::Skipped);
    }

    let pid = process::id().to_string();
    let log_label = runtime.label();

    if let Some(value) =
        should_skip_action_matching("MOON_SKIP_INSTALL_DEPS", get_skip_key(runtime, project))
    {
        debug!(
            env = value,
            "Skipping {} dependency install because {} is set",
            log_label,
            color::symbol("MOON_SKIP_INSTALL_DEPS")
        );

        return Ok(ActionStatus::Skipped);
    }

    if proto_core::is_offline() {
        debug!("No internet connection, skipping install");

        return Ok(ActionStatus::Skipped);
    }

    if env::var("INTERNAL_MOON_INSTALLING_DEPS").is_ok_and(|other_pid| other_pid != pid) {
        debug!("Detected another dependency install running, skipping install");

        return Ok(ActionStatus::Skipped);
    }

    // When cache is write only, avoid install as user is typically force updating cache
    if app_context.cache_engine.is_write_only() {
        debug!("Force updating cache, skipping install");

        return Ok(ActionStatus::Skipped);
    }

    // When running against affected files, avoid install as it interrupts the workflow
    if action_context.affected_only {
        debug!("Running against affected files, skipping install");

        return Ok(ActionStatus::Skipped);
    }

    let registry = PlatformManager::read();
    let platform = registry.get(runtime)?;

    let Some((lockfile_name, manifest_name)) = platform.get_dependency_configs()? else {
        debug!("No dependency manager configured for language, skipping install");

        return Ok(ActionStatus::Skipped);
    };

    // Hash dependencies from all applicable manifests
    let manifests_hash = hash_manifests(
        action,
        &action_context,
        &app_context,
        project,
        platform,
        &manifest_name,
    )
    .await?;

    // Extract lockfile timestamp
    let lockfile_timestamp = track_lockfile(&app_context, project, &lockfile_name)?;

    // Only install deps if a cache miss
    let mut state = app_context
        .cache_engine
        .state
        .load_state::<DependenciesCacheState>(get_state_path(&app_context, runtime, project))?;

    if manifests_hash != state.data.last_hash
        || lockfile_timestamp == 0
        || lockfile_timestamp > state.data.last_install_time
    {
        let working_dir = project.map_or(&app_context.workspace_root, |proj| &proj.root);

        // To avoid nested installs caused by child processes, we set this environment
        // variable with the current process ID and compare against it. If the IDs are
        // the same then multiple installs are happening in parallel in the same
        // process (via the pipeline), otherwise it's a child process.
        env::set_var("INTERNAL_MOON_INSTALLING_DEPS", pid);

        debug!(
            "Installing {} dependencies in {}",
            log_label,
            color::path(working_dir)
        );

        action.operations.extend(
            platform
                .install_deps(&action_context, runtime, working_dir)
                .await?,
        );

        state.data.last_hash = manifests_hash;
        state.data.last_install_time = now_millis();
        state.save()?;

        return Ok(ActionStatus::Passed);
    }

    debug!("Lockfile or manifests have not changed since last run, skipping install",);

    Ok(ActionStatus::Skipped)
}

async fn hash_manifests(
    action: &mut Action,
    action_context: &ActionContext,
    app_context: &AppContext,
    project: Option<&Project>,
    platform: &BoxedPlatform,
    manifest_name: &str,
) -> miette::Result<String> {
    let mut operation = Operation::hash_generation();
    let mut hasher = app_context.cache_engine.hash.create_hasher(&action.label);
    let root_manifest = app_context.workspace_root.join(manifest_name);

    // Always include the root manifest
    if root_manifest.exists() {
        platform
            .hash_manifest_deps(
                &root_manifest,
                &mut hasher,
                &app_context.workspace_config.hasher,
            )
            .await?;
    }

    // When running in the project root, include their manifest
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
    // When running in the workspace root, account for nested manifests
    else {
        for touched_file in &action_context.touched_files {
            if touched_file.ends_with(manifest_name) {
                let nested_manifest = touched_file.to_path(&app_context.workspace_root);

                platform
                    .hash_manifest_deps(
                        &nested_manifest,
                        &mut hasher,
                        &app_context.workspace_config.hasher,
                    )
                    .await?;
            }
        }
    }

    let hash = app_context.cache_engine.hash.save_manifest(hasher)?;

    operation.meta.set_hash(&hash);
    operation.finish(ActionStatus::Passed);

    action.operations.push(operation);

    Ok(hash)
}

fn track_lockfile(
    app_context: &AppContext,
    project: Option<&Project>,
    lockfile_name: &str,
) -> miette::Result<u128> {
    let mut lockfile_path = app_context.workspace_root.join(lockfile_name);

    // Check if the project has their own lockfile
    if let Some(project) = project {
        let project_lockfile = project.root.join(lockfile_name);

        if project_lockfile.exists() {
            lockfile_path = project_lockfile;
        }
    }

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
    let state_path = PathBuf::from(format!("deps-{}.json", encode_component(runtime.id())));

    if let Some(project) = project {
        return app_context
            .cache_engine
            .state
            .get_project_dir(&project.id)
            .join(state_path);
    }

    state_path
}
