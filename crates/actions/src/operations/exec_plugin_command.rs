use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf, encode_component};
use moon_env_var::GlobalEnvBag;
use moon_hash::hash_content;
use moon_pdk_api::{CacheInput, ExecCommand, ExecCommandInput, VirtualPath};
use moon_process::{Command, Output};
use moon_time::to_millis;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::warn;

hash_content!(
    struct ExecCommandHash<'data> {
        command: &'data ExecCommandInput,

        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        input_env: BTreeMap<String, String>,

        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        input_files: BTreeMap<WorkspaceRelativePathBuf, String>,
    }
);

pub type OnExecFn = Arc<dyn Fn(&ExecCommandInput) + Send + Sync>;

#[derive(Clone, Default)]
pub struct ExecCommandOptions {
    pub on_exec: Option<OnExecFn>,
    pub prefix: String,
}

pub async fn exec_plugin_command(
    app_context: Arc<AppContext>,
    command: &ExecCommand,
    options: &ExecCommandOptions,
) -> miette::Result<Output> {
    let mut cmd = create_process_command_from_plugin(&command.command);
    cmd.with_console(app_context.console.clone());

    options.on_exec.as_ref().inspect(|on_exec| {
        on_exec(&command.command);
    });

    let output = if command.command.stream {
        cmd.exec_stream_output().await?
    } else {
        cmd.exec_capture_output().await?
    };

    Ok(output)
}

pub async fn exec_plugin_command_with_cache(
    app_context: Arc<AppContext>,
    command: &ExecCommand,
    options: &ExecCommandOptions,
) -> miette::Result<Option<Output>> {
    let Some(key) = &command.cache else {
        return exec_plugin_command(app_context, command, options)
            .await
            .map(Some);
    };

    let mut hash_item = ExecCommandHash {
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
            format!("{}:{}", options.prefix, encode_component(key)),
            hash_item,
            async move || exec_plugin_command(app_context, command, options).await,
        )
        .await
}

pub async fn exec_plugin_commands(
    app_context: Arc<AppContext>,
    commands: Vec<ExecCommand>,
    options: ExecCommandOptions,
) -> miette::Result<Vec<Output>> {
    let mut serial = vec![];
    let mut parallel = vec![];
    let mut outputs = vec![];

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
        if let Some(output) =
            exec_plugin_command_with_cache(app_context.clone(), &command, &options).await?
        {
            outputs.push(output);
        }
    }

    // Then execute the parallel commands
    let mut set = JoinSet::new();

    for command in parallel {
        let app_context = app_context.clone();
        let options = options.clone();

        set.spawn(
            async move { exec_plugin_command_with_cache(app_context, &command, &options).await },
        );
    }

    while let Some(result) = set.join_next().await {
        if let Some(output) = result.into_diagnostic()?? {
            outputs.push(output);
        }
    }

    Ok(outputs)
}

fn create_process_command_from_plugin(input: &ExecCommandInput) -> Command {
    let mut command = Command::new(&input.command);
    command.args(&input.args);
    command.envs(&input.env);

    if let Some(cwd) = input.working_dir.as_ref().and_then(|dir| dir.real_path()) {
        command.cwd(cwd);
    }

    command
}

async fn gather_cache_inputs(
    app_context: &AppContext,
    inputs: &[CacheInput],
    hash_item: &mut ExecCommandHash<'_>,
) -> miette::Result<()> {
    let mut hashable_files = vec![];

    let get_file = |virtual_path: &VirtualPath,
                    workspace_root: &Path|
     -> Option<(PathBuf, WorkspaceRelativePathBuf)> {
        if let Some(abs_path) = virtual_path.real_path() {
            if let Ok(rel_path) = abs_path.relative_to(workspace_root) {
                if abs_path.exists() {
                    return Some((abs_path, rel_path));
                } else {
                    // Don't warn for missing files
                    return None;
                }
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
                if let Some((_, rel_path)) = get_file(virtual_path, &app_context.workspace_root) {
                    hashable_files.push(rel_path);
                }
            }
            CacheInput::FileSize(virtual_path) | CacheInput::FileTimestamp(virtual_path) => {
                if let Some((abs_path, rel_path)) =
                    get_file(virtual_path, &app_context.workspace_root)
                {
                    let metadata = fs::metadata(&abs_path)?;

                    if matches!(input, CacheInput::FileSize(_)) {
                        hash_item
                            .input_files
                            .insert(rel_path, format!("size:{}", metadata.len()));
                    } else if let Ok(timestamp) =
                        metadata.modified().or_else(|_| metadata.created())
                    {
                        hash_item
                            .input_files
                            .insert(rel_path, format!("timestamp:{}", to_millis(timestamp)));
                    }
                }
            }
        };
    }

    if !hashable_files.is_empty() && app_context.vcs.is_enabled() {
        for (rel_path, hash) in app_context
            .vcs
            .get_file_hashes(&hashable_files, true)
            .await?
        {
            hash_item
                .input_files
                .insert(rel_path, format!("hash:{hash}"));
        }
    }

    Ok(())
}
