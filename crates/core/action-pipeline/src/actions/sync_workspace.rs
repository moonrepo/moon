use super::should_skip_action;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_actions::{sync_codeowners, sync_vcs_hooks};
use moon_logger::debug;
use moon_process::Command;
use moon_project_graph::ProjectGraph;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::PROTO_CLI_VERSION;
use moon_utils::is_test_env;
use moon_workspace::Workspace;
use proto_core::download_from_url_to_file;
use starbase_styles::color;
use starbase_utils::fs;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:sync-workspace";

pub async fn sync_workspace(
    _action: &mut Action,
    _context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    project_graph: Arc<RwLock<ProjectGraph>>,
) -> miette::Result<ActionStatus> {
    // This causes a lot of churn in tests, revisit
    if !is_test_env() {
        env::set_var("MOON_RUNNING_ACTION", "sync-workspace");
    }

    let workspace = workspace.read().await;
    let project_graph = project_graph.read().await;

    // Install proto
    install_proto(&workspace).await?;

    // Sync workspace
    debug!(target: LOG_TARGET, "Syncing workspace");

    if should_skip_action("MOON_SKIP_SYNC_WORKSPACE") {
        debug!(
            target: LOG_TARGET,
            "Skipping sync workspace action because MOON_SKIP_SYNC_WORKSPACE is set",
        );

        return Ok(ActionStatus::Skipped);
    }

    if workspace.config.codeowners.sync_on_run {
        debug!(
            target: LOG_TARGET,
            "Syncing code owners ({} enabled)",
            color::property("codeowners.syncOnRun"),
        );

        sync_codeowners(&workspace, &project_graph, false).await?;
    }

    if workspace.config.vcs.sync_hooks {
        debug!(
            target: LOG_TARGET,
            "Syncing {} hooks ({} enabled)",
            workspace.config.vcs.manager,
            color::property("vcs.syncHooks"),
        );

        sync_vcs_hooks(&workspace, false).await?;
    }

    Ok(ActionStatus::Passed)
}

async fn install_proto(workspace: &Workspace) -> miette::Result<()> {
    let install_dir = workspace
        .proto_env
        .tools_dir
        .join("proto")
        .join(PROTO_CLI_VERSION);

    if install_dir.exists() {
        return Ok(());
    }

    debug!(target: LOG_TARGET, "Installing proto");

    print_checkpoint(
        format!("installing proto {}", PROTO_CLI_VERSION),
        Checkpoint::Setup,
    );

    let script_name = if cfg!(windows) {
        "proto.ps1"
    } else {
        "proto.sh"
    };
    let script_file = workspace.proto_env.temp_dir.join(script_name);
    let script_url = format!("https://moonrepo.dev/install/{script_name}");

    debug!("Downloading from {}", script_url);

    download_from_url_to_file(
        &script_url,
        &script_file,
        workspace.proto_env.get_http_client()?,
    )
    .await?;

    fs::update_perms(&script_file, None)?;

    debug!("Executing install script {}", script_file.display());

    let mut cmd = Command::new(script_file);
    let mut cmd = cmd
        .arg(PROTO_CLI_VERSION)
        .env("PROTO_INSTALL_DIR", install_dir)
        .env("PROTO_DEBUG", "true")
        .with_shell()
        .create_async();

    if env::var("MOON_DEBUG_PROTO_INSTALL").is_ok() {
        cmd.exec_stream_output().await?;
    } else {
        cmd.exec_capture_output().await?;
    }

    Ok(())
}
