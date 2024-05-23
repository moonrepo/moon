use super::should_skip_action_matching;
use miette::IntoDiagnostic;
use moon_action::{Action, ActionStatus, Operation, OperationMeta};
use moon_action_context::ActionContext;
use moon_cache_item::cache_item;
use moon_logger::{debug, warn};
use moon_platform::{PlatformManager, Runtime};
use moon_project::Project;
use moon_utils::time;
use moon_workspace::Workspace;
use starbase_styles::color;
use starbase_utils::fs;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

cache_item!(
    pub struct DependenciesState {
        pub last_hash: String,
        pub last_install_time: u128,
    }
);

const LOG_TARGET: &str = "moon:action:install-deps";

// We need to include the runtime and project in the key,
// since moon can install deps in multiple projects across multiple tools
// all in parallel!
fn get_installation_key(runtime: &Runtime, project: Option<&Project>) -> String {
    format!(
        "{}:{}",
        runtime,
        match project {
            Some(p) => p.id.as_ref(),
            None => "*",
        }
    )
}

pub async fn install_deps(
    action: &mut Action,
    context: Arc<ActionContext>,
    workspace: Arc<Workspace>,
    runtime: &Runtime,
    project: Option<&Project>,
) -> miette::Result<ActionStatus> {
    env::set_var("MOON_RUNNING_ACTION", "install-deps");

    if runtime.platform.is_system() {
        return Ok(ActionStatus::Skipped);
    }

    let install_key = get_installation_key(runtime, project);

    if proto_core::is_offline() {
        warn!(
            target: LOG_TARGET,
            "No internet connection, skipping install"
        );

        return Ok(ActionStatus::Skipped);
    }

    if should_skip_action_matching("MOON_SKIP_INSTALL_DEPS", &install_key) {
        debug!(
            target: LOG_TARGET,
            "Skipping install deps action because MOON_SKIP_INSTALL_DEPS is set",
        );

        return Ok(ActionStatus::Skipped);
    }

    // When the install is happening as a child process of another install, avoid recursion
    if env::var("MOON_INSTALLING_DEPS").unwrap_or_default() == install_key {
        debug!(
            target: LOG_TARGET,
            "Detected another install running, skipping install"
        );

        return Ok(ActionStatus::Skipped);
    }

    // When cache is write only, avoid install as user is typically force updating cache
    if workspace.cache_engine.get_mode().is_write_only() {
        debug!(target: LOG_TARGET, "Force updating cache, skipping install");

        return Ok(ActionStatus::Skipped);
    }

    // When running against affected files, avoid install as it interrupts the workflow
    if context.affected_only {
        debug!(
            target: LOG_TARGET,
            "Running against affected files, skipping install",
        );

        return Ok(ActionStatus::Skipped);
    }

    let registry = PlatformManager::read();
    let platform = registry.get(runtime)?;

    let Some((lockfile, manifest)) = platform.get_dependency_configs()? else {
        debug!(
            target: LOG_TARGET,
            "No dependency manager for language, skipping install",
        );

        return Ok(ActionStatus::Skipped);
    };

    // Determine the working directory and whether lockfiles and manifests have been modified
    let mut operation = Operation::new(OperationMeta::hash_generation());

    let working_dir = project.map(|p| &p.root).unwrap_or_else(|| &workspace.root);
    let manifest_path = working_dir.join(&manifest);
    let lockfile_path = working_dir.join(&lockfile);
    let mut hasher = workspace
        .cache_engine
        .hash
        .create_hasher(format!("Install {} deps", runtime.label()));
    let mut last_modified = 0;

    if manifest_path.exists() {
        platform
            .hash_manifest_deps(&manifest_path, &mut hasher, &workspace.config.hasher)
            .await?;
    }

    if lockfile_path.exists() {
        last_modified =
            time::to_millis(fs::metadata(&lockfile_path)?.modified().into_diagnostic()?);
    }

    // When running in the workspace root, account for nested manifests
    if project.is_none() {
        for touched_file in &context.touched_files {
            if touched_file.ends_with(&manifest) {
                platform
                    .hash_manifest_deps(
                        &touched_file.to_path(""),
                        &mut hasher,
                        &workspace.config.hasher,
                    )
                    .await?;
            }
        }
    }

    // Install dependencies in the working directory
    let hash = workspace.cache_engine.hash.save_manifest(hasher)?;

    operation.meta.set_hash(&hash);
    operation.finish(ActionStatus::Passed);

    action.operations.push(operation);

    let state_path = format!("deps{runtime}.json");
    let mut state = workspace
        .cache_engine
        .state
        .load_state::<DependenciesState>(if let Some(project) = &project {
            workspace
                .cache_engine
                .state
                .get_project_dir(&project.id)
                .join(state_path)
        } else {
            PathBuf::from(state_path)
        })?;

    if hash != state.data.last_hash
        || last_modified == 0
        || last_modified > state.data.last_install_time
    {
        env::set_var("MOON_INSTALLING_DEPS", install_key);

        debug!(
            target: LOG_TARGET,
            "Installing {} dependencies in {}",
            runtime.label(),
            color::path(working_dir)
        );

        let operations = platform
            .install_deps(&context, runtime, working_dir)
            .await?;

        state.data.last_hash = hash;
        state.data.last_install_time = time::now_millis();
        state.save()?;

        action.operations.extend(operations);

        env::remove_var("MOON_INSTALLING_DEPS");

        return Ok(ActionStatus::Passed);
    }

    debug!(
        target: LOG_TARGET,
        "Lockfile has not changed since last install, skipping install",
    );

    Ok(ActionStatus::Skipped)
}
