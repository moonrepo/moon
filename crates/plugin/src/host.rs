use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use moon_common::{color, Id};
use moon_env::MoonEnvironment;
use moon_project_graph::ProjectGraph;
use moon_target::{Target, TargetScope};
use proto_core::ProtoEnvironment;
use std::fmt;
use std::sync::Arc;
use tracing::{instrument, trace};

#[derive(Clone, Default)]
pub struct PluginHostData {
    pub moon_env: Arc<MoonEnvironment>,
    pub project_graph: Arc<ProjectGraph>,
    pub proto_env: Arc<ProtoEnvironment>,
}

unsafe impl Send for PluginHostData {}
unsafe impl Sync for PluginHostData {}

impl fmt::Debug for PluginHostData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("moon_env", &self.moon_env)
            .field("proto_env", &self.proto_env)
            .finish()
    }
}

pub fn create_moon_host_functions(data: PluginHostData) -> Vec<Function> {
    vec![
        Function::new(
            "load_project",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data.clone()),
            load_project,
        ),
        Function::new(
            "load_task",
            [ValType::I64],
            [ValType::I64],
            UserData::new(data),
            load_task,
        ),
    ]
}

fn map_error(error: miette::Report) -> Error {
    Error::msg(error.to_string())
}

// #[instrument(name = "host_load_project", skip_all)]
fn load_project(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<PluginHostData>,
) -> Result<(), Error> {
    let id_raw: String = plugin.memory_get_val(&inputs[0])?;
    let id = Id::new(id_raw)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        project_id = id.as_str(),
        "Calling host function {}",
        color::label("load_project"),
    );

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let project = data.project_graph.get(&id).map_err(map_error)?;

    trace!(
        plugin = &uuid,
        project_id = id.as_str(),
        "Called host function {}",
        color::label("load_project"),
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&project)?)?;

    Ok(())
}

// #[instrument(name = "host_load_task", skip_all)]
fn load_task(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<PluginHostData>,
) -> Result<(), Error> {
    let target_raw: String = plugin.memory_get_val(&inputs[0])?;
    let target = Target::parse(&target_raw).map_err(map_error)?;
    let uuid = plugin.id().to_string();

    trace!(
        plugin = &uuid,
        target = target.as_str(),
        "Calling host function {}",
        color::label("load_task"),
    );

    let TargetScope::Project(project_id) = &target.scope else {
        return Err(Error::msg(
            "Unable to load task. Requires a fully-qualified target with a project scope.",
        ));
    };

    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let project = data.project_graph.get(&project_id).map_err(map_error)?;
    let task = project.get_task(&target.task_id).map_err(map_error)?;

    trace!(
        plugin = &uuid,
        target = target.as_str(),
        "Called host function {}",
        color::label("load_task"),
    );

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(task)?)?;

    Ok(())
}
