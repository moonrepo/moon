use miette::IntoDiagnostic;
use moon_action::{ActionStatus, Operation};
use moon_app_context::AppContext;
use moon_args::join_args;
use moon_common::{
    Id, color,
    path::{PathExt, WorkspaceRelativePathBuf, encode_component},
};
use moon_console::{Checkpoint, Console};
use moon_env_var::GlobalEnvBag;
use moon_hash::hash_content;
use moon_pdk_api::{CacheInput, ExecCommand, ExecCommandInput, VirtualPath};
use moon_process::{Command, Output};
use moon_project::Project;
use moon_time::to_millis;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, warn};

hash_content!(
    struct ExecCommandHash<'data> {
        key: &'data str,

        command: &'data ExecCommandInput,

        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        input_env: BTreeMap<String, String>,

        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        input_files: BTreeMap<WorkspaceRelativePathBuf, String>,
    }
);

pub type OnExecFn = Arc<dyn Fn(&ExecCommand, (u8, u8)) -> miette::Result<()> + Send + Sync>;

pub fn handle_on_exec(
    console: &Console,
    command: &ExecCommand,
    attempts: (u8, u8),
) -> miette::Result<()> {
    let input = &command.command;
    let label = command
        .label
        .clone()
        .unwrap_or_else(|| format!("{} {}", input.command, join_args(&input.args)));

    if attempts.0 > 1 {
        console.print_checkpoint_with_comments(
            Checkpoint::Setup,
            label,
            [format!("attempt {} of {}", attempts.0, attempts.1)],
        )
    } else {
        console.print_checkpoint(Checkpoint::Setup, label)
    }
}

#[derive(Clone, Default)]
pub struct ExecCommandOptions {
    pub on_exec: Option<OnExecFn>,
    pub prefix: String,
    pub project: Option<Arc<Project>>,
    pub working_dir: Option<PathBuf>,
}

async fn internal_exec_plugin_command(
    app_context: Arc<AppContext>,
    command: &ExecCommand,
    options: &ExecCommandOptions,
    attempts: (u8, u8),
) -> miette::Result<Output> {
    let input = &command.command;

    let mut cmd = Command::new(&input.command);
    cmd.args(&input.args);
    cmd.envs(&input.env);

    if let Some(cwd) = input.cwd.as_ref().and_then(|dir| dir.real_path()) {
        cmd.cwd(cwd);
    } else if let Some(cwd) = &options.working_dir {
        cmd.cwd(cwd);
    }

    cmd.with_console(app_context.console.clone());
    cmd.set_error_on_nonzero(!command.allow_failure);
    cmd.set_print_command(app_context.workspace_config.pipeline.log_running_command);

    // Must be last!
    let toolchain_registry = &app_context.toolchain_registry;

    if let Some(project) = &options.project {
        toolchain_registry
            .augment_command_for_project(&mut cmd, GlobalEnvBag::instance(), &project.config)
            .await?;
    } else {
        toolchain_registry
            .augment_command_for_workspace(&mut cmd, GlobalEnvBag::instance())
            .await?;
    }

    if let Some(on_exec) = &options.on_exec {
        on_exec(command, attempts)?;
    }

    if command.command.stream {
        cmd.exec_stream_output().await
    } else {
        cmd.exec_capture_output().await
    }
}

async fn internal_exec_plugin_command_as_operation(
    app_context: Arc<AppContext>,
    command: &ExecCommand,
    options: &ExecCommandOptions,
    attempts: (u8, u8),
) -> miette::Result<Operation> {
    let mut op = Operation::process_execution(&command.command.command); // hah

    let result = match &command.cache {
        Some(key) => {
            let mut hash_item = ExecCommandHash {
                key,
                command: &command.command,
                input_env: BTreeMap::new(),
                input_files: BTreeMap::new(),
            };

            if !command.inputs.is_empty() {
                gather_cache_inputs(&app_context, &command.inputs, &mut hash_item).await?;
            }

            app_context
                .clone()
                .cache_engine
                .execute_if_changed(
                    format!(
                        "{}:{}",
                        options.prefix,
                        encode_component(key).to_lowercase()
                    ),
                    hash_item,
                    async move |_| {
                        internal_exec_plugin_command(app_context, command, options, attempts).await
                    },
                )
                .await
        }
        None => internal_exec_plugin_command(app_context, command, options, attempts)
            .await
            .map(Some),
    };

    match result {
        Ok(maybe_output) => {
            if let Some(output) = maybe_output {
                op.finish_from_output(output.status(), output.stdout, output.stderr);
            } else {
                op.finish(ActionStatus::Skipped);
            }

            Ok(op)
        }
        Err(error) => {
            op.finish(ActionStatus::Failed);

            Err(error)
        }
    }
}

pub async fn exec_plugin_command(
    app_context: Arc<AppContext>,
    command: &ExecCommand,
    options: &ExecCommandOptions,
) -> miette::Result<Vec<Operation>> {
    let mut ops = vec![];
    let attempt_count = 1 + command.retry_count;

    for attempt in 1..=attempt_count {
        match internal_exec_plugin_command_as_operation(
            app_context.clone(),
            command,
            options,
            (attempt, attempt_count),
        )
        .await
        {
            Ok(op) => {
                let success = op.status == ActionStatus::Skipped
                    || op
                        .get_exec_output()
                        .is_some_and(|exec| exec.get_exit_code() == 0);

                ops.push(op);

                if success {
                    break;
                }
            }
            Err(error) => {
                if attempt == attempt_count {
                    return Err(error);
                }
            }
        };

        debug!(
            "Failed to execute {} command, retrying...",
            color::shell(command.label.as_ref().unwrap_or(&command.command.command)),
        );
    }

    Ok(ops)
}

pub async fn exec_plugin_commands(
    toolchain_id: &str,
    app_context: Arc<AppContext>,
    commands: Vec<ExecCommand>,
    options: ExecCommandOptions,
) -> miette::Result<Vec<Operation>> {
    let mut serial = vec![];
    let mut parallel = vec![];
    let mut ops = vec![];

    for command in commands {
        if command.parallel {
            parallel.push(command);
        } else {
            serial.push(command);
        }
    }

    // Execute serial first, as a parallel command may
    // depend on a serial command having been executed
    for command in serial {
        ops.extend(exec_plugin_command(app_context.clone(), &command, &options).await?);
    }

    // Then execute the parallel commands
    if !parallel.is_empty() {
        let mut set = JoinSet::new();

        for command in parallel {
            let app_context = app_context.clone();
            let options = options.clone();

            set.spawn(async move { exec_plugin_command(app_context, &command, &options).await });
        }

        while let Some(result) = set.join_next().await {
            ops.extend(result.into_diagnostic()??);
        }
    }

    // Inherit toolchain ID
    for op in &mut ops {
        op.plugin = Some(Id::new(toolchain_id)?);
    }

    Ok(ops)
}

async fn gather_cache_inputs(
    app_context: &AppContext,
    inputs: &[CacheInput],
    hash_item: &mut ExecCommandHash<'_>,
) -> miette::Result<()> {
    let mut hash_files = vec![];
    let mut size_files = vec![];
    let has_vcs = app_context.vcs.is_enabled();

    let get_file = |virtual_path: &VirtualPath,
                    workspace_root: &Path|
     -> Option<(PathBuf, WorkspaceRelativePathBuf)> {
        if let Some(abs_path) = virtual_path.real_path()
            && let Ok(rel_path) = abs_path.relative_to(workspace_root)
        {
            if abs_path.exists() {
                return Some((abs_path, rel_path));
            } else {
                // Don't warn for missing files
                return None;
            }
        }

        warn!(
            path = virtual_path.to_string(),
            "Only real paths within the workspace can be used as a cache input, received an invalid virtual path",
        );

        None
    };

    for input in inputs {
        match input {
            CacheInput::EnvVar(name) => {
                hash_item.input_env.insert(
                    name.into(),
                    GlobalEnvBag::instance().get(name).unwrap_or_default(),
                );
            }
            CacheInput::FileHash(virtual_path) => {
                if let Some(res) = get_file(virtual_path, &app_context.workspace_root) {
                    if has_vcs {
                        hash_files.push(res);
                    } else {
                        size_files.push(res);
                    }
                }
            }
            CacheInput::FileSize(virtual_path) => {
                if let Some(res) = get_file(virtual_path, &app_context.workspace_root) {
                    size_files.push(res);
                }
            }
            CacheInput::FileTimestamp(virtual_path) => {
                if let Some((abs_path, rel_path)) =
                    get_file(virtual_path, &app_context.workspace_root)
                {
                    let metadata = fs::metadata(&abs_path)?;

                    if let Ok(timestamp) = metadata.modified().or_else(|_| metadata.created()) {
                        hash_item
                            .input_files
                            .insert(rel_path, format!("timestamp:{}", to_millis(timestamp)));
                    }
                }
            }
        };
    }

    if !hash_files.is_empty() {
        let hash_files = hash_files.into_iter().map(|f| f.1).collect::<Vec<_>>();

        for (rel_path, hash) in app_context.vcs.get_file_hashes(&hash_files, true).await? {
            hash_item
                .input_files
                .insert(rel_path, format!("hash:{hash}"));
        }
    }

    if !size_files.is_empty() {
        for (abs_path, rel_path) in size_files {
            hash_item
                .input_files
                .insert(rel_path, format!("size:{}", fs::metadata(&abs_path)?.len()));
        }
    }

    Ok(())
}
