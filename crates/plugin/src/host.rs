use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use moon_common::{Id, color};
use moon_config::{ProjectToolchainEntry, ToolchainConfig, ToolchainPluginConfig, WorkspaceConfig};
use moon_env::MoonEnvironment;
use moon_target::Target;
use moon_workspace_graph::WorkspaceGraph;
use proto_core::ProtoEnvironment;
use rustc_hash::FxHashMap;
use std::fmt;
use std::sync::{Arc, OnceLock};
use tracing::{instrument, trace};
use warpgate::host::{HostData, create_host_functions as create_shared_host_functions};

#[derive(Clone, Default)]
pub struct MoonHostData {
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,
    pub toolchain_config: Arc<ToolchainConfig>,
    pub workspace_config: Arc<WorkspaceConfig>,
    pub workspace_graph: Arc<OnceLock<Arc<WorkspaceGraph>>>,
}

impl fmt::Debug for MoonHostData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MoonHostData")
            .field("moon_env", &self.moon_env)
            .field("proto_env", &self.proto_env)
            .field("toolchain_config", &self.toolchain_config)
            .field("workspace_config", &self.workspace_config)
            .finish()
    }
}

pub fn create_host_functions(data: MoonHostData, shared_data: HostData) -> Vec<Function> {
    let mut functions = vec![];
    functions.extend(create_shared_host_functions(shared_data));
    functions.extend(vec![
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

    match &project_id {
        Some(project_id) => {
            let workspace_graph = data.workspace_graph.get().unwrap();
            let project = workspace_graph.get_project(project_id).map_err(map_error)?;

            let default_config = ToolchainPluginConfig::default();
            let config = project
                .config
                .toolchain
                .get_plugin_config(&toolchain_id)
                .and_then(|entry| match entry {
                    ProjectToolchainEntry::Config(cfg) => Some(cfg),
                    _ => None,
                })
                .unwrap_or(&default_config);

            plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&config.to_json())?)?;
        }
        None => {
            let config = data
                .toolchain_config
                .get_plugin_config(&toolchain_id)
                .ok_or_else(|| {
                    Error::msg(format!(
                        "Unable to load toolchain configuration. Toolchain {toolchain_id} does not exist."
                    ))
                })?;

            plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&config.to_json())?)?;
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
