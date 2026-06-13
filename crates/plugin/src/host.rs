use crate::PluginType;
use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use moon_common::{Id, color};
use moon_config::{
    ExtensionsConfig, ProjectToolchainEntry, ToolchainPluginConfig, ToolchainsConfig,
    WorkspaceConfig,
};
use moon_env::MoonEnvironment;
use moon_pdk_api::{
    Id as ProcessCapabilityId, ProcessCapabilityDeclaration, ProcessCommandInput,
    ProcessCommandResult, ProcessOutputChunk, ProcessOutputChunkInput, ProcessOutputCloseInput,
    ProcessOutputCloseOutput, ProcessOutputStream,
};
use moon_target::Target;
use moon_workspace_graph::WorkspaceGraph;
use proto_core::ProtoEnvironment;
use rustc_hash::FxHashMap;
use starbase_utils::json::merge as json_merge;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::process::{Output, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::{instrument, trace};
use warpgate::{
    api::convert_to_real_native_path,
    host::{HostData, create_host_functions as create_shared_host_functions},
};

const PROCESS_OUTPUT_CHUNK_SIZE: usize = 64 * 1024;

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

#[derive(Clone, Debug, Default)]
pub struct ProcessHostAccess {
    configuration: Arc<Mutex<Option<ProcessHostConfiguration>>>,
}

#[derive(Debug)]
struct ProcessHostConfiguration {
    executables: FxHashMap<ProcessCapabilityId, PathBuf>,
    workspace_root: PathBuf,
}

impl ProcessHostAccess {
    pub fn configure(
        &self,
        capabilities: &[ProcessCapabilityDeclaration],
        workspace_root: &Path,
    ) -> miette::Result<()> {
        let mut configuration = self
            .configuration
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        if configuration.is_some() {
            return Err(miette::miette!(
                "plugin process capabilities are already configured"
            ));
        }

        let workspace_root = workspace_root.canonicalize().map_err(|error| {
            miette::miette!(
                "failed to resolve plugin workspace `{}`: {error}",
                workspace_root.display()
            )
        })?;

        if !workspace_root.is_dir() {
            return Err(miette::miette!(
                "plugin workspace `{}` is not a directory",
                workspace_root.display()
            ));
        }

        let mut resolved = FxHashMap::default();

        for capability in capabilities {
            if resolved.contains_key(&capability.id) {
                return Err(miette::miette!(
                    "process capability `{}` is declared more than once",
                    capability.id
                ));
            }

            if !is_valid_executable_name(&capability.executable) {
                return Err(miette::miette!(
                    "process capability `{}` must declare an executable name without a path",
                    capability.id
                ));
            }

            let executable = resolve_process_executable(&capability.executable, &workspace_root)
                .ok_or_else(|| {
                    miette::miette!(
                        "unable to resolve executable `{}` for process capability `{}`",
                        capability.executable,
                        capability.id
                    )
                })?;

            resolved.insert(capability.id.clone(), executable);
        }

        *configuration = Some(ProcessHostConfiguration {
            executables: resolved,
            workspace_root,
        });

        Ok(())
    }

    fn executable(&self, capability: &ProcessCapabilityId) -> Result<PathBuf, Error> {
        self.configuration
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .ok_or_else(|| Error::msg("plugin process capabilities are not configured"))?
            .executables
            .get(capability)
            .cloned()
            .ok_or_else(|| {
                Error::msg(format!(
                    "plugin requested undeclared process capability `{capability}`"
                ))
            })
    }

    fn workspace_root(&self) -> Result<PathBuf, Error> {
        self.configuration
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .map(|configuration| configuration.workspace_root.clone())
            .ok_or_else(|| Error::msg("plugin process capabilities are not configured"))
    }
}

#[derive(Clone)]
struct ProcessHostData {
    access: ProcessHostAccess,
    execution: Arc<Mutex<()>>,
    output: Arc<Mutex<ProcessOutputState>>,
    shared: HostData,
}

#[derive(Default)]
struct ProcessOutputState {
    next_id: u64,
    output: Option<ProcessOutput>,
}

impl ProcessOutputState {
    fn reserve_result(&mut self) -> Result<u64, Error> {
        self.output = None;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| Error::msg("process result IDs are exhausted"))?;

        Ok(self.next_id)
    }

    fn close_result(&mut self, result_id: u64) -> Result<(), Error> {
        if !self
            .output
            .as_ref()
            .is_some_and(|output| output.id == result_id)
        {
            return Err(Error::msg(format!("unknown process result `{result_id}`")));
        }

        self.output = None;

        Ok(())
    }
}

struct ProcessOutput {
    id: u64,
    stderr: Vec<u8>,
    stdout: Vec<u8>,
}

impl ProcessOutput {
    fn read(&self, stream: ProcessOutputStream, offset: u64) -> Result<ProcessOutputChunk, Error> {
        let output = match stream {
            ProcessOutputStream::Stderr => &self.stderr,
            ProcessOutputStream::Stdout => &self.stdout,
        };
        let len = u64::try_from(output.len())?;

        if offset > len {
            return Err(Error::msg(format!(
                "process output offset {offset} exceeds stream length {len}"
            )));
        }

        let start = usize::try_from(offset)?;
        let end = start
            .saturating_add(PROCESS_OUTPUT_CHUNK_SIZE)
            .min(output.len());

        Ok(ProcessOutputChunk::from_bytes(
            &output[start..end],
            end == output.len(),
        ))
    }
}

pub(crate) fn create_host_functions(
    plugin_type: PluginType,
    data: MoonHostData,
    shared_data: HostData,
    process_access: Option<ProcessHostAccess>,
) -> Vec<Function> {
    let mut functions = create_shared_host_functions(shared_data.clone());

    if matches!(plugin_type, PluginType::Vcs) {
        functions.retain(|function| function.name() == "host_log");
        let process_data = ProcessHostData {
            access: process_access.expect("VCS plugins require process host access"),
            execution: Arc::new(Mutex::new(())),
            output: Arc::new(Mutex::new(ProcessOutputState::default())),
            shared: shared_data,
        };

        functions.extend([
            Function::new(
                "exec_process_command_v1",
                [ValType::I64],
                [ValType::I64],
                UserData::new(process_data.clone()),
                exec_process_command_v1,
            ),
            Function::new(
                "read_process_output_v1",
                [ValType::I64],
                [ValType::I64],
                UserData::new(process_data.clone()),
                read_process_output_v1,
            ),
            Function::new(
                "close_process_output_v1",
                [ValType::I64],
                [ValType::I64],
                UserData::new(process_data),
                close_process_output_v1,
            ),
        ]);

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

fn is_valid_executable_name(executable: &str) -> bool {
    if executable.is_empty()
        || executable == "."
        || executable == ".."
        || executable.contains('\0')
        || executable.contains(['/', '\\'])
    {
        return false;
    }

    let is_single_component = matches!(
        Path::new(executable).components().next(),
        Some(Component::Normal(name)) if name == executable
    ) && Path::new(executable).components().count() == 1;
    let lowercase = executable.to_ascii_lowercase();

    is_single_component
        && (!cfg!(windows) || !(lowercase.ends_with(".bat") || lowercase.ends_with(".cmd")))
}

fn resolve_process_executable(executable: &str, workspace_root: &Path) -> Option<PathBuf> {
    resolve_process_executable_in_path(executable, workspace_root, &std::env::var_os("PATH")?)
}

fn resolve_process_executable_in_path(
    executable: &str,
    workspace_root: &Path,
    path: &std::ffi::OsStr,
) -> Option<PathBuf> {
    for directory in std::env::split_paths(path).filter(|path| path.is_absolute()) {
        let candidate = if !std::env::consts::EXE_SUFFIX.is_empty()
            && !executable
                .to_ascii_lowercase()
                .ends_with(std::env::consts::EXE_SUFFIX)
        {
            directory.join(format!("{executable}{}", std::env::consts::EXE_SUFFIX))
        } else {
            directory.join(executable)
        };

        if !is_executable_file(&candidate) {
            continue;
        }

        let Ok(candidate) = candidate.canonicalize() else {
            continue;
        };

        if !candidate.starts_with(workspace_root) {
            return Some(candidate);
        }
    }

    None
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn exec_process_command_v1(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<ProcessHostData>,
) -> Result<(), Error> {
    let input: ProcessCommandInput = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;
    let data = user_data.get()?;
    let data = data.lock().unwrap_or_else(|error| error.into_inner());
    let _execution = data
        .execution
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let result_id = data
        .output
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .reserve_result()?;
    let cwd = input
        .cwd
        .as_ref()
        .map(|path| convert_to_real_native_path(path, &data.shared.virtual_paths))
        .unwrap_or_else(|| data.shared.working_dir.clone());
    let workspace_root = data.access.workspace_root()?;
    let cwd = cwd.canonicalize().map_err(|error| {
        Error::msg(format!(
            "failed to resolve plugin process working directory `{}`: {error}",
            cwd.display()
        ))
    })?;

    validate_process_cwd(&cwd, &workspace_root)?;

    let executable = data.access.executable(&input.capability)?;
    let result = execute_process_command(&executable, &cwd, &input)?;
    let output = ProcessCommandResult {
        result_id,
        exit_code: result.status.code().unwrap_or(-1),
        stdout_len: u64::try_from(result.stdout.len())?,
        stderr_len: u64::try_from(result.stderr.len())?,
    };

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&output)?)?;

    data.output
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .output = Some(ProcessOutput {
        id: result_id,
        stderr: result.stderr,
        stdout: result.stdout,
    });

    Ok(())
}

fn execute_process_command(
    executable: &Path,
    cwd: &Path,
    input: &ProcessCommandInput,
) -> Result<Output, Error> {
    std::process::Command::new(executable)
        .current_dir(cwd)
        .args(&input.args)
        .envs(&input.env)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            Error::msg(format!(
                "failed to execute process capability `{}` with `{}`: {error}",
                input.capability,
                executable.display()
            ))
        })
}

fn read_process_output_v1(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<ProcessHostData>,
) -> Result<(), Error> {
    let input: ProcessOutputChunkInput = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;
    let data = user_data.get()?;
    let data = data.lock().unwrap_or_else(|error| error.into_inner());
    let state = data
        .output
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let output = state
        .output
        .as_ref()
        .filter(|output| output.id == input.result_id)
        .ok_or_else(|| Error::msg(format!("unknown process result `{}`", input.result_id)))?;
    let chunk = output.read(input.stream, input.offset)?;

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&chunk)?)?;

    Ok(())
}

fn close_process_output_v1(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<ProcessHostData>,
) -> Result<(), Error> {
    let input: ProcessOutputCloseInput = serde_json::from_str(plugin.memory_get_val(&inputs[0])?)?;
    let data = user_data.get()?;
    let data = data.lock().unwrap_or_else(|error| error.into_inner());
    let mut state = data
        .output
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.close_result(input.result_id)?;
    plugin.memory_set_val(
        &mut outputs[0],
        serde_json::to_string(&ProcessOutputCloseOutput {})?,
    )?;

    Ok(())
}

fn validate_process_cwd(cwd: &Path, workspace_root: &Path) -> Result<(), Error> {
    if !cwd.is_dir() {
        return Err(Error::msg(format!(
            "plugin process working directory `{}` is not a directory",
            cwd.display()
        )));
    }

    if !cwd.starts_with(workspace_root) {
        return Err(Error::msg(
            "plugin process working directory must be inside the workspace",
        ));
    }

    Ok(())
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
mod process_host_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn capability(id: &str, executable: &str) -> ProcessCapabilityDeclaration {
        ProcessCapabilityDeclaration {
            id: ProcessCapabilityId::raw(id),
            executable: executable.into(),
        }
    }

    fn process_input(args: &[&str]) -> ProcessCommandInput {
        ProcessCommandInput {
            capability: ProcessCapabilityId::raw("git"),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            cwd: None,
            env: BTreeMap::new(),
        }
    }

    fn resolve_git(workspace_root: &Path) -> PathBuf {
        resolve_process_executable("git", &workspace_root.canonicalize().unwrap())
            .expect("Git is a repository prerequisite")
    }

    fn host_data(workspace_root: &Path) -> HostData {
        HostData {
            cache_dir: workspace_root.join("cache"),
            http_client: Arc::new(warpgate::create_http_client().unwrap()),
            virtual_paths: vec![(workspace_root.to_path_buf(), PathBuf::from("/workspace"))],
            working_dir: workspace_root.to_path_buf(),
        }
    }

    #[test]
    fn restricts_process_functions_to_vcs_plugins() {
        let workspace = starbase_sandbox::create_empty_sandbox();
        let moon_data = MoonHostData {
            moon_env: Arc::new(MoonEnvironment::new_testing(workspace.path())),
            proto_env: Arc::new(ProtoEnvironment::new_testing(workspace.path()).unwrap()),
            ..Default::default()
        };
        let mut vcs_functions = create_host_functions(
            PluginType::Vcs,
            moon_data.clone(),
            host_data(workspace.path()),
            Some(ProcessHostAccess::default()),
        )
        .into_iter()
        .map(|function| function.name().to_owned())
        .collect::<Vec<_>>();
        vcs_functions.sort();

        assert_eq!(
            vcs_functions,
            [
                "close_process_output_v1",
                "exec_process_command_v1",
                "host_log",
                "read_process_output_v1",
            ]
        );

        let extension_functions = create_host_functions(
            PluginType::Extension,
            moon_data,
            host_data(workspace.path()),
            None,
        );
        assert!(
            extension_functions
                .iter()
                .all(|function| !function.name().contains("process_"))
        );
        assert!(
            extension_functions
                .iter()
                .any(|function| function.name() == "exec_command")
        );
    }

    #[test]
    fn validates_executable_names() {
        assert!(is_valid_executable_name("git"));
        assert!(is_valid_executable_name("git.exe"));

        for invalid in ["", ".", "..", "../git", "bin/git", "bin\\git"] {
            assert!(!is_valid_executable_name(invalid), "{invalid}");
        }

        if cfg!(windows) {
            assert!(!is_valid_executable_name("git.bat"));
            assert!(!is_valid_executable_name("git.cmd"));
        }
    }

    #[test]
    fn configures_capabilities_once_and_atomically() {
        let workspace = starbase_sandbox::create_empty_sandbox();
        let access = ProcessHostAccess::default();

        assert!(access.executable(&ProcessCapabilityId::raw("git")).is_err());
        assert!(
            access
                .configure(
                    &[capability("git", "git"), capability("git", "git")],
                    workspace.path(),
                )
                .is_err()
        );
        access
            .configure(&[capability("git", "git")], workspace.path())
            .unwrap();
        assert!(access.executable(&ProcessCapabilityId::raw("git")).is_ok());
        assert!(
            access
                .executable(&ProcessCapabilityId::raw("undeclared"))
                .is_err()
        );
        assert!(
            access
                .configure(&[capability("other", "git")], workspace.path())
                .is_err()
        );
    }

    #[test]
    fn empty_configuration_authorizes_nothing() {
        let workspace = starbase_sandbox::create_empty_sandbox();
        let access = ProcessHostAccess::default();

        access.configure(&[], workspace.path()).unwrap();

        assert!(access.executable(&ProcessCapabilityId::raw("git")).is_err());
        assert!(access.configure(&[], workspace.path()).is_err());
    }

    #[test]
    fn rejects_missing_and_path_executables() {
        let workspace = starbase_sandbox::create_empty_sandbox();

        for executable in ["../git", "bin/git", "definitely-not-a-moon-executable"] {
            assert!(
                ProcessHostAccess::default()
                    .configure(&[capability("vcs", executable)], workspace.path())
                    .is_err(),
                "{executable}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn ignores_relative_path_entries_and_non_executable_files() {
        use std::os::unix::fs::PermissionsExt;

        let workspace = starbase_sandbox::create_empty_sandbox();
        let tools = starbase_sandbox::create_empty_sandbox();
        let tool = tools.path().join("moon-test-tool");
        std::fs::write(&tool, "not executable").unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o644)).unwrap();
        let workspace_root = workspace.path().canonicalize().unwrap();

        assert!(
            resolve_process_executable_in_path(
                "moon-test-tool",
                &workspace_root,
                &std::env::join_paths([PathBuf::from("relative"), tools.path().to_path_buf()])
                    .unwrap(),
            )
            .is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_executables_reached_through_symlinks_into_workspace() {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let workspace = starbase_sandbox::create_empty_sandbox();
        let tools = starbase_sandbox::create_empty_sandbox();
        let workspace_tool = workspace.path().join("moon-test-tool");
        std::fs::write(&workspace_tool, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&workspace_tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        symlink(&workspace_tool, tools.path().join("moon-test-tool")).unwrap();

        assert!(
            resolve_process_executable_in_path(
                "moon-test-tool",
                &workspace.path().canonicalize().unwrap(),
                &std::env::join_paths([tools.path()]).unwrap(),
            )
            .is_none()
        );
    }

    #[cfg(windows)]
    #[test]
    fn validates_the_exe_path_that_windows_will_execute() {
        use std::os::windows::fs::symlink_file;

        let workspace = starbase_sandbox::create_empty_sandbox();
        let tools = starbase_sandbox::create_empty_sandbox();
        let extensionless_tool = tools.path().join("moon-test-tool");
        let workspace_tool = workspace.path().join("moon-test-tool.exe");
        std::fs::write(&extensionless_tool, "outside workspace").unwrap();
        std::fs::write(&workspace_tool, "inside workspace").unwrap();
        symlink_file(&workspace_tool, tools.path().join("moon-test-tool.exe")).unwrap();

        assert!(
            resolve_process_executable_in_path(
                "moon-test-tool",
                &workspace.path().canonicalize().unwrap(),
                &std::env::join_paths([tools.path()]).unwrap(),
            )
            .is_none()
        );
    }

    #[test]
    fn confines_process_working_directories_to_workspace() {
        let parent = starbase_sandbox::create_empty_sandbox();
        let outside = starbase_sandbox::create_empty_sandbox();
        let workspace = parent.path().join("workspace");
        let sibling = parent.path().join("workspace-sibling");
        let nested = workspace.join("nested");
        let file = workspace.join("file");
        std::fs::create_dir(&workspace).unwrap();
        std::fs::create_dir(&sibling).unwrap();
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(&file, "file").unwrap();

        assert!(validate_process_cwd(&workspace, &workspace).is_ok());
        assert!(validate_process_cwd(&nested, &workspace).is_ok());
        assert!(validate_process_cwd(&sibling, &workspace).is_err());
        assert!(validate_process_cwd(outside.path(), &workspace).is_err());
        assert!(validate_process_cwd(&file, &workspace).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_working_directory_symlink_escapes() {
        use std::os::unix::fs::symlink;

        let workspace = starbase_sandbox::create_empty_sandbox();
        let outside = starbase_sandbox::create_empty_sandbox();
        let link = workspace.path().join("outside");
        symlink(outside.path(), &link).unwrap();

        assert!(
            validate_process_cwd(
                &link.canonicalize().unwrap(),
                &workspace.path().canonicalize().unwrap(),
            )
            .is_err()
        );
    }

    #[test]
    fn executes_without_shells_and_with_closed_stdin() {
        let workspace = starbase_sandbox::create_empty_sandbox();
        let git = resolve_git(workspace.path());
        let input = process_input(&["hash-object", "--stdin"]);

        let output = execute_process_command(&git, workspace.path(), &input).unwrap();

        assert!(output.status.success());
        assert!(!String::from_utf8(output.stdout).unwrap().trim().is_empty());
    }

    #[test]
    fn preserves_nonzero_exit_and_separate_streams() {
        let workspace = starbase_sandbox::create_empty_sandbox();
        let git = resolve_git(workspace.path());
        let input = process_input(&["definitely-not-a-git-command"]);

        let output = execute_process_command(&git, workspace.path(), &input).unwrap();

        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }

    #[test]
    fn applies_environment_overrides() {
        let workspace = starbase_sandbox::create_empty_sandbox();
        let git = resolve_git(workspace.path());
        let mut input = process_input(&["var", "GIT_EDITOR"]);
        input
            .env
            .insert("GIT_EDITOR".into(), "moon-test-editor".into());

        let output = execute_process_command(&git, workspace.path(), &input).unwrap();

        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "moon-test-editor"
        );
    }

    #[test]
    fn reports_spawn_failures_without_environment_values() {
        let workspace = starbase_sandbox::create_empty_sandbox();
        let mut input = process_input(&[]);
        input.env.insert("MOON_SECRET".into(), "do-not-leak".into());
        let error = execute_process_command(
            &workspace.path().join("missing-executable"),
            workspace.path(),
            &input,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("missing-executable"), "{error}");
        assert!(!error.contains("do-not-leak"), "{error}");
    }

    #[test]
    fn reads_output_in_byte_safe_chunks() {
        let mut bytes = vec![b'x'; PROCESS_OUTPUT_CHUNK_SIZE];
        bytes.extend([0, 0xff, b'z']);
        let output = ProcessOutput {
            id: 1,
            stderr: vec![],
            stdout: bytes,
        };

        let first = output.read(ProcessOutputStream::Stdout, 0).unwrap();
        assert_eq!(first.decode().unwrap().len(), PROCESS_OUTPUT_CHUNK_SIZE);
        assert!(!first.eof);

        let second = output
            .read(
                ProcessOutputStream::Stdout,
                PROCESS_OUTPUT_CHUNK_SIZE as u64,
            )
            .unwrap();
        assert_eq!(second.decode().unwrap(), [0, 0xff, b'z']);
        assert!(second.eof);

        let end = output
            .read(
                ProcessOutputStream::Stdout,
                (PROCESS_OUTPUT_CHUNK_SIZE + 3) as u64,
            )
            .unwrap();
        assert!(end.decode().unwrap().is_empty());
        assert!(end.eof);
        assert!(
            output
                .read(
                    ProcessOutputStream::Stdout,
                    (PROCESS_OUTPUT_CHUNK_SIZE + 4) as u64,
                )
                .is_err()
        );

        let stderr = output.read(ProcessOutputStream::Stderr, 0).unwrap();
        assert!(stderr.decode().unwrap().is_empty());
        assert!(stderr.eof);

        let exact = ProcessOutput {
            id: 2,
            stderr: vec![],
            stdout: vec![b'x'; PROCESS_OUTPUT_CHUNK_SIZE],
        }
        .read(ProcessOutputStream::Stdout, 0)
        .unwrap();
        assert_eq!(exact.decode().unwrap().len(), PROCESS_OUTPUT_CHUNK_SIZE);
        assert!(exact.eof);
    }

    #[test]
    fn invalidates_closes_and_never_reuses_process_results() {
        let mut state = ProcessOutputState::default();
        let first = state.reserve_result().unwrap();
        state.output = Some(ProcessOutput {
            id: first,
            stderr: vec![],
            stdout: vec![1],
        });
        let second = state.reserve_result().unwrap();

        assert_eq!(first, 1);
        assert_eq!(second, 2);
        assert!(state.output.is_none());

        state.output = Some(ProcessOutput {
            id: second,
            stderr: vec![],
            stdout: vec![2],
        });
        assert!(state.close_result(first).is_err());
        state.close_result(second).unwrap();
        assert!(state.close_result(second).is_err());

        state.next_id = u64::MAX;
        state.output = Some(ProcessOutput {
            id: second,
            stderr: vec![],
            stdout: vec![3],
        });
        assert!(state.reserve_result().is_err());
        assert!(state.output.is_none());
    }
}
