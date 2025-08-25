use extism::{CurrentPlugin, Error, UserData, Val};
use serde_json::{Value, json};
use std::sync::Arc;

#[allow(clippy::enum_variant_names)]
#[derive(PartialEq)]
pub enum MoonHostFunction {
    LoadProject,
    LoadProjects,
    LoadTask,
    LoadTasks,
    LoadToolchainConfig,
}

impl MoonHostFunction {
    pub fn as_str(&self) -> &str {
        match self {
            Self::LoadProject => "load_project_by_id",
            Self::LoadProjects => "load_projects_by_id",
            Self::LoadTask => "load_task_by_target",
            Self::LoadTasks => "load_tasks_by_target",
            Self::LoadToolchainConfig => "load_toolchain_config_by_id",
        }
    }
}

pub type LoadProjectHostFunc = Arc<dyn Fn(String) -> Value>;
pub type LoadProjectsHostFunc = Arc<dyn Fn(Vec<String>) -> Value>;
pub type LoadTaskHostFunc = Arc<dyn Fn(String) -> Value>;
pub type LoadTasksHostFunc = Arc<dyn Fn(Vec<String>) -> Value>;
pub type LoadToolchainConfigHostFunc = Arc<dyn Fn(String, Option<String>) -> Value>;

#[derive(Clone, Default)]
pub struct MockedHostFuncs {
    load_project: Option<LoadProjectHostFunc>,
    load_projects: Option<LoadProjectsHostFunc>,
    load_task: Option<LoadTaskHostFunc>,
    load_tasks: Option<LoadTasksHostFunc>,
    load_toolchain_config: Option<LoadToolchainConfigHostFunc>,
}

impl MockedHostFuncs {
    pub fn mock_load_project(&mut self, func: impl Fn(String) -> Value + 'static) {
        self.load_project = Some(Arc::new(func));
    }

    pub fn mock_load_projects(&mut self, func: impl Fn(Vec<String>) -> Value + 'static) {
        self.load_projects = Some(Arc::new(func));
    }

    pub fn mock_load_task(&mut self, func: impl Fn(String) -> Value + 'static) {
        self.load_task = Some(Arc::new(func));
    }

    pub fn mock_load_tasks(&mut self, func: impl Fn(Vec<String>) -> Value + 'static) {
        self.load_tasks = Some(Arc::new(func));
    }

    pub fn mock_load_toolchain_config(
        &mut self,
        func: impl Fn(String, Option<String>) -> Value + 'static,
    ) {
        self.load_toolchain_config = Some(Arc::new(func));
    }
}

pub fn mocked_host_func_impl(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<(MoonHostFunction, MockedHostFuncs)>,
) -> Result<(), Error> {
    let data = user_data.get()?;
    let data = data.lock().unwrap();
    let (func_type, mocked_funcs) = &*data;

    let value = match func_type {
        MoonHostFunction::LoadProject => {
            let id: String = plugin.memory_get_val(&inputs[0])?;

            mocked_funcs
                .load_project
                .as_ref()
                .map_or(json!({}), |func| func(id))
        }
        MoonHostFunction::LoadProjects => {
            let ids_raw: String = plugin.memory_get_val(&inputs[0])?;
            let ids: Vec<String> = serde_json::from_str(&ids_raw)?;

            mocked_funcs
                .load_projects
                .as_ref()
                .map_or(json!({}), |func| func(ids))
        }
        MoonHostFunction::LoadTask => {
            let id: String = plugin.memory_get_val(&inputs[0])?;

            mocked_funcs
                .load_task
                .as_ref()
                .map_or(json!({}), |func| func(id))
        }
        MoonHostFunction::LoadTasks => {
            let ids_raw: String = plugin.memory_get_val(&inputs[0])?;
            let ids: Vec<String> = serde_json::from_str(&ids_raw)?;

            mocked_funcs
                .load_tasks
                .as_ref()
                .map_or(json!({}), |func| func(ids))
        }
        MoonHostFunction::LoadToolchainConfig => {
            let toolchain_id: String = plugin.memory_get_val(&inputs[0])?;
            let project_id = match inputs.get(1) {
                Some(input) => Some(plugin.memory_get_val::<String>(input)?),
                None => None,
            };

            mocked_funcs
                .load_toolchain_config
                .as_ref()
                .map_or(json!({}), |func| func(toolchain_id, project_id))
        }
    };

    plugin.memory_set_val(&mut outputs[0], serde_json::to_string(&value)?)?;

    Ok(())
}
