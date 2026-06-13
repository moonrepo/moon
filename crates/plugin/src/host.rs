use crate::PluginType;
use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use moon_common::{Id, color};
use moon_config::{
    ExtensionsConfig, ProjectToolchainEntry, ToolchainPluginConfig, ToolchainsConfig,
    WorkspaceConfig,
};
use moon_env::MoonEnvironment;
use moon_target::Target;
use moon_workspace_graph::WorkspaceGraph;
use proto_core::ProtoEnvironment;
use rustc_hash::FxHashMap;
use starbase_utils::json::merge as json_merge;
use std::fmt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, OnceLock};
use tracing::{instrument, trace};
use warpgate::{from_virtual_path, host::HostData};
use warpgate_api::{ExecCommandInput, ExecCommandOutput};

#[derive(Clone, Default)]
pub struct MoonHostData {
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,
    pub extensions_config: Arc<ExtensionsConfig>,
    pub toolchains_config: Arc<ToolchainsConfig>,
    pub workspace_config: Arc<WorkspaceConfig>,
    pub workspace_graph: Arc<OnceLock<Arc<WorkspaceGraph>>>,
}

impl fmt::Debug for MoonHostData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MoonHostData")
            .field("moon_env", &self.moon_env)
            .field("proto_env", &self.proto_env)
            .field("extensions_config", &self.extensions_config)
            .field("toolchains_config", &self.toolchains_config)
            .field("workspace_config", &self.workspace_config)
            .finish()
    }
}

#[derive(Clone)]
struct VcsHostData {
    shared: HostData,
    workspace_root: PathBuf,
}

pub fn create_host_functions(
    plugin_type: PluginType,
    data: MoonHostData,
    shared_data: HostData,
) -> Vec<Function> {
    let mut functions = warpgate::host::create_host_functions(shared_data.clone());

    if matches!(plugin_type, PluginType::Vcs) {
        functions.retain(|function| function.name() == "host_log");
        functions.push(Function::new(
            "exec_command",
            [ValType::I64],
            [ValType::I64],
            UserData::new(VcsHostData {
                shared: shared_data,
                workspace_root: data.moon_env.workspace_root.clone(),
            }),
            exec_vcs_command,
        ));

        return functions;
    }

    functions.extend(vec![
        Function::new(
            "load_extension_config_by_id",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            load_extension_config_by_id,
        ),
        Function::new(
            "load_project_by_id",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            load_project,
        ),
        Function::new(
            "load_projects_by_id",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            load_projects,
        ),
        Function::new(
            "load_task_by_target",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            load_task,
        ),
        Function::new(
            "load_tasks_by_target",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            load_tasks,
        ),
        Function::new(
            "load_toolchain_config_by_id",
            [ValType::I64, ValType::I64],
            [ValType::I64],
            UserData::new(data),
            load_toolchain_config_by_id,
        ),
    ]);
    functions
}

fn exec_vcs_command(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<VcsHostData>,
) -> Result<(), Error> {
    let input: ExecCommandInput = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;
    validate_vcs_command(&input)?;

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let cwd = input
        .cwd
        .as_ref()
        .map(|path| from_virtual_path(&data.shared.virtual_paths, path))
        .unwrap_or_else(|| data.shared.working_dir.clone());
    let workspace_root = data.workspace_root.canonicalize()?;
    let cwd = cwd.canonicalize()?;

    if !cwd.starts_with(&workspace_root) {
        return Err(Error::msg(
            "VCS plugin command working directory must be inside the workspace",
        ));
    }

    let mut command = Command::new(&input.command);
    command.args(&input.args).current_dir(cwd);

    if input.command == "git" {
        command
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_PAGER", "")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_COUNT", "2")
            .env("GIT_CONFIG_KEY_0", "core.fsmonitor")
            .env("GIT_CONFIG_VALUE_0", "false")
            .env("GIT_CONFIG_KEY_1", "status.relativePaths")
            .env("GIT_CONFIG_VALUE_1", "false");
    }

    let result = command.output()?;
    let output = ExecCommandOutput {
        command: input.command,
        exit_code: result.status.code().unwrap_or(-1),
        stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
        stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
        streamed: false,
    };

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&output)?)?;

    Ok(())
}

fn validate_vcs_command(input: &ExecCommandInput) -> Result<(), Error> {
    if input.shell.is_some()
        || input.stream
        || input.set_executable
        || !input.env.is_empty()
        || !input.paths.is_empty()
    {
        return Err(Error::msg(
            "VCS plugins may only execute a guarded piped command without host overrides",
        ));
    }

    match input.command.as_str() {
        "git" => validate_git_command(&input.args),
        "jj" => validate_jj_command(&input.args),
        command => Err(Error::msg(format!(
            "VCS plugins may not execute `{command}`"
        ))),
    }
}

fn validate_git_command(args: &[String]) -> Result<(), Error> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(Error::msg("VCS plugin git command has no subcommand"));
    };

    if subcommand == "--version" && args.len() == 1 {
        return Ok(());
    }

    if args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "-C" | "-c"
                | "--config-env"
                | "--exec-path"
                | "--git-dir"
                | "--html-path"
                | "--info-path"
                | "--man-path"
                | "--namespace"
                | "--paginate"
                | "--super-prefix"
                | "--work-tree"
                | "--ext-diff"
                | "--no-index"
                | "--output"
                | "--textconv"
        ) || arg.starts_with("--config-env=")
            || arg.starts_with("--exec-path=")
            || arg.starts_with("--git-dir=")
            || arg.starts_with("--namespace=")
            || arg.starts_with("--output=")
            || arg.starts_with("--super-prefix=")
            || arg.starts_with("--work-tree=")
    }) {
        return Err(Error::msg(
            "VCS plugin git command contains a forbidden option",
        ));
    }

    match subcommand {
        "branch" if matches!(args, [command, option] if command == "branch" && option == "--show-current") => {
            Ok(())
        }
        "remote" if matches!(args, [command, operation, _] if command == "remote" && operation == "get-url") => {
            Ok(())
        }
        "diff" | "diff-tree" | "ls-files" | "ls-tree" | "merge-base" | "rev-list" | "rev-parse"
        | "status" => Ok(()),
        "config" if valid_git_hooks_config(args) => Ok(()),
        _ => Err(Error::msg(format!(
            "VCS plugins may not execute `git {subcommand}`"
        ))),
    }
}

fn valid_git_hooks_config(args: &[String]) -> bool {
    matches!(
        args,
        [command, key, value]
            if command == "config" && key == "core.hooksPath" && valid_git_hooks_path(value)
    ) || matches!(
        args,
        [command, worktree, key, value]
            if command == "config"
                && worktree == "--worktree"
                && key == "core.hooksPath"
                && valid_git_hooks_path(value)
    ) || matches!(
        args,
        [command, unset, key]
            if command == "config" && unset == "--unset" && key == "core.hooksPath"
    ) || matches!(
        args,
        [command, worktree, unset, key]
            if command == "config"
                && worktree == "--worktree"
                && unset == "--unset"
                && key == "core.hooksPath"
    ) || matches!(
        args,
        [command, key, value]
            if command == "config"
                && key == "extensions.worktreeConfig"
                && value == "true"
    )
}

fn valid_git_hooks_path(path: &str) -> bool {
    (path == ".moon/hooks"
        || path.ends_with("/.moon/hooks")
        || path == ".config/moon/hooks"
        || path.ends_with("/.config/moon/hooks"))
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && !path.split('/').any(|part| matches!(part, "" | "." | ".."))
}

fn validate_jj_command(args: &[String]) -> Result<(), Error> {
    if matches!(args, [option] if option == "--version") {
        return Ok(());
    }

    if args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--config" | "--config-file" | "--repository" | "-R" | "--tool"
        ) || arg.starts_with("--config=")
            || arg.starts_with("--config-file=")
            || arg.starts_with("--repository=")
            || arg.starts_with("-R")
            || arg.starts_with("--tool=")
    }) {
        return Err(Error::msg(
            "VCS plugin jj command contains a forbidden option",
        ));
    }

    let mut args = args.iter();
    let mut isolated_operation = false;
    let subcommand = loop {
        let Some(arg) = args.next() else {
            return Err(Error::msg("VCS plugin jj command has no subcommand"));
        };

        if arg == "--ignore-working-copy" || arg.starts_with("--at-operation=") {
            continue;
        }

        if arg == "--no-integrate-operation" {
            isolated_operation = true;
            continue;
        }

        break arg.as_str();
    };

    match subcommand {
        "root" | "log" | "diff" => Ok(()),
        "op" if args.next().is_some_and(|arg| arg == "log") => Ok(()),
        "new" if isolated_operation => Ok(()),
        _ => Err(Error::msg(format!(
            "VCS plugins may not execute `jj {subcommand}`"
        ))),
    }
}

fn map_error(error: miette::Report) -> Error {
    Error::msg(error.to_string())
}

#[instrument(name = "host_load_project_by_id", skip_all)]
fn load_project(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<MoonHostData>,
) -> Result<(), Error> {
    let id_raw: String = plugin.memory_get_val(&inputs[0])?;
    let id = Id::new(id_raw)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        project_id = id.as_str(),
        "Calling host function {}",
        color::label("load_project_by_id"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let project = data
        .workspace_graph
        .get()
        .unwrap()
        .get_project(&id)
        .map_err(map_error)?;

    trace!(
        plugin = &uuid,
        project_id = id.as_str(),
        "Called host function {}",
        color::label("load_project_by_id"),
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&project)?)?;

    Ok(())
}

#[instrument(name = "host_load_projects_by_id", skip_all)]
fn load_projects(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<MoonHostData>,
) -> Result<(), Error> {
    let ids_raw: String = plugin.memory_get_val(&inputs[0])?;
    let ids: Vec<String> = serde_json::from_str(&ids_raw)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        project_ids = ?ids,
        "Calling host function {}",
        color::label("load_projects_by_id"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let workspace_graph = data.workspace_graph.get().unwrap();
    let mut projects = FxHashMap::default();

    for id in &ids {
        let id = Id::raw(id);
        let project = workspace_graph.get_project(&id).map_err(map_error)?;

        projects.insert(id, project);
    }

    trace!(
        plugin = &uuid,
        project_ids = ?ids,
        "Called host function {}",
        color::label("load_projects_by_id"),
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&projects)?)?;

    Ok(())
}

#[instrument(name = "host_load_task_by_target", skip_all)]
fn load_task(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<MoonHostData>,
) -> Result<(), Error> {
    let target_raw: String = plugin.memory_get_val(&inputs[0])?;
    let target = Target::parse(&target_raw).map_err(map_error)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        task_target = target.as_str(),
        "Calling host function {}",
        color::label("load_task_by_target"),
    );

    if target.get_project_id().is_err() {
        return Err(Error::msg(format!(
            "Unable to load task {target}. Requires a fully-qualified target with a project scope."
        )));
    };

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let task = data
        .workspace_graph
        .get()
        .unwrap()
        .get_task(&target)
        .map_err(map_error)?;

    trace!(
        plugin = &uuid,
        task_target = target.as_str(),
        "Called host function {}",
        color::label("load_task_by_target"),
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&task)?)?;

    Ok(())
}

#[instrument(name = "host_load_tasks_by_target", skip_all)]
fn load_tasks(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<MoonHostData>,
) -> Result<(), Error> {
    let targets_raw: String = plugin.memory_get_val(&inputs[0])?;
    let targets: Vec<String> = serde_json::from_str(&targets_raw)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        task_targets = ?targets,
        "Calling host function {}",
        color::label("load_tasks_by_target"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let workspace_graph = data.workspace_graph.get().unwrap();
    let mut tasks = FxHashMap::default();

    for target in &targets {
        let target = Target::parse(target).map_err(map_error)?;

        if target.get_project_id().is_err() {
            return Err(Error::msg(format!(
                "Unable to load task {target}. Requires a fully-qualified target with a project scope."
            )));
        };

        let task = workspace_graph.get_task(&target).map_err(map_error)?;

        tasks.insert(target, task);
    }

    trace!(
        plugin = &uuid,
        task_targets = ?targets,
        "Called host function {}",
        color::label("load_tasks_by_target"),
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&tasks)?)?;

    Ok(())
}

#[instrument(name = "host_load_extension_config_by_id", skip_all)]
fn load_extension_config_by_id(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<MoonHostData>,
) -> Result<(), Error> {
    let uuid = plugin.id().to_string();
    let extension_id = Id::new(plugin.memory_get_val::<String>(&inputs[0])?)?;

    trace!(
        plugin = &uuid,
        extension_id = extension_id.as_str(),
        "Calling host function {}",
        color::label("load_extension_config_by_id"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    let config = data
        .extensions_config
        .get_plugin_config(&extension_id)
        .ok_or_else(|| {
            Error::msg(format!(
                "Unable to load extension configuration. Extension {extension_id} does not exist."
            ))
        })?;

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&config.to_json())?)?;

    trace!(
        plugin = &uuid,
        extension_id = extension_id.as_str(),
        "Called host function {}",
        color::label("load_extension_config_by_id"),
    );

    Ok(())
}

#[instrument(name = "host_load_toolchain_config_by_id", skip_all)]
fn load_toolchain_config_by_id(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<MoonHostData>,
) -> Result<(), Error> {
    let uuid = plugin.id().to_string();
    let toolchain_id = Id::new(plugin.memory_get_val::<String>(&inputs[0])?)?;
    let mut project_id = None;

    if let Some(input) = inputs.get(1) {
        let id = plugin.memory_get_val::<String>(input)?;

        // Extism passes it through as empty
        if !id.is_empty() {
            project_id.replace(Id::new(id)?);
        }
    }

    trace!(
        plugin = &uuid,
        project_id = project_id.as_ref().map(|id| id.as_str()),
        toolchain_id = toolchain_id.as_str(),
        "Calling host function {}",
        color::label("load_toolchain_config_by_id"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();

    let default_config = ToolchainPluginConfig::default();
    let root_config = data
        .toolchains_config
        .get_plugin_config(&toolchain_id)
        .ok_or_else(|| {
            Error::msg(format!(
                "Unable to load toolchain configuration. Toolchain {toolchain_id} does not exist."
            ))
        })?;

    match &project_id {
        Some(project_id) => {
            let workspace_graph = data.workspace_graph.get().unwrap();
            let project = workspace_graph.get_project(project_id).map_err(map_error)?;

            let config = project
                .config
                .toolchains
                .get_plugin_config(&toolchain_id)
                .and_then(|entry| match entry {
                    ProjectToolchainEntry::Object(cfg) => Some(cfg),
                    _ => None,
                })
                .unwrap_or(&default_config);

            // We don't have access to the toolchain registry here,
            // so we must manually merge these config objects
            plugin.memory_set_val(
                &mut outputs[0],
                serde_json::to_string(&json_merge(&root_config.to_json(), &config.to_json()))?,
            )?;
        }
        None => {
            plugin.memory_set_val(
                &mut outputs[0],
                serde_json::to_string(&root_config.to_json())?,
            )?;
        }
    };

    trace!(
        plugin = &uuid,
        project_id = project_id.as_ref().map(|id| id.as_str()),
        toolchain_id = toolchain_id.as_str(),
        "Called host function {}",
        color::label("load_toolchain_config_by_id"),
    );

    Ok(())
}

#[cfg(test)]
mod vcs_host_tests {
    use super::*;

    #[test]
    fn allows_read_only_jj_operations() {
        for args in [
            vec!["--version"],
            vec!["--ignore-working-copy", "root"],
            vec!["--at-operation=abc", "log", "-r", "@"],
            vec!["--at-operation=abc", "diff", "-r", "@"],
            vec!["op", "log", "-n", "1"],
            vec![
                "--at-operation=abc",
                "--no-integrate-operation",
                "new",
                "left",
                "right",
            ],
        ] {
            assert!(validate_vcs_command(&ExecCommandInput::pipe("jj", args)).is_ok());
        }
    }

    #[test]
    fn allows_required_git_operations() {
        for args in [
            vec!["--version"],
            vec!["branch", "--show-current"],
            vec!["diff", "--name-status", "base", "head"],
            vec!["diff-tree", "--root", "head"],
            vec!["ls-files", "--stage", "-z"],
            vec!["ls-tree", "head", "submodule"],
            vec!["merge-base", "base", "head"],
            vec!["remote", "get-url", "origin"],
            vec!["rev-list", "--parents", "head"],
            vec!["rev-parse", "--show-toplevel"],
            vec!["status", "--porcelain=v1"],
            vec!["config", "--worktree", "core.hooksPath", ".moon/hooks"],
            vec![
                "config",
                "--worktree",
                "core.hooksPath",
                "workspace/.moon/hooks",
            ],
            vec!["config", "core.hooksPath", ".config/moon/hooks"],
            vec!["config", "extensions.worktreeConfig", "true"],
        ] {
            assert!(validate_vcs_command(&ExecCommandInput::pipe("git", args)).is_ok());
        }
    }

    #[test]
    fn rejects_mutating_or_escaping_git_operations() {
        for args in [
            vec!["branch", "--delete", "main"],
            vec!["remote", "add", "origin", "https://example.com/repo"],
            vec!["diff", "--ext-diff", "base", "head"],
            vec!["diff", "--no-index", "/etc/passwd", "/dev/null"],
            vec!["diff", "--output=/tmp/leak", "base", "head"],
            vec!["config", "core.hooksPath", "../.moon/hooks"],
            vec!["config", "core.hooksPath", "/tmp/.moon/hooks"],
        ] {
            assert!(validate_vcs_command(&ExecCommandInput::pipe("git", args)).is_err());
        }
    }

    #[test]
    fn rejects_other_commands_and_mutating_jj_operations() {
        assert!(validate_vcs_command(&ExecCommandInput::pipe("sh", ["-c", "true"])).is_err());
        assert!(
            validate_vcs_command(&ExecCommandInput::pipe("jj", ["util", "exec", "sh"])).is_err()
        );
        assert!(validate_vcs_command(&ExecCommandInput::pipe("jj", ["new"])).is_err());
    }

    #[test]
    fn rejects_jj_configuration_and_external_tools() {
        assert!(
            validate_vcs_command(&ExecCommandInput::pipe(
                "jj",
                ["log", "--config", "aliases.x='util exec sh'"],
            ))
            .is_err()
        );
        assert!(
            validate_vcs_command(&ExecCommandInput::pipe("jj", ["diff", "--tool=malicious"],))
                .is_err()
        );
        assert!(
            validate_vcs_command(&ExecCommandInput::pipe("jj", ["root", "-R/tmp/external"],))
                .is_err()
        );
    }
}
