use crate::contexts::ProjectContext;
use moon_common::{is_ci, is_docker};
use moon_config::Input;
use moon_env_var::GlobalEnvBag;
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use std::sync::Arc;
use tera::{Error, Function, Kwargs, State, TeraResult, Value};

// TODO? changed_files, affected_files

// STANDARD

pub fn env(args: Kwargs, _state: &State) -> TeraResult<String> {
    let key = args.must_get::<&str>("var")?;

    match GlobalEnvBag::instance().get(key) {
        Some(value) => Ok(value),
        None => Ok(args.get::<&str>("fallback")?.unwrap_or_default().into()),
    }
}

pub fn if_ci(args: Kwargs, _state: &State) -> TeraResult<Value> {
    if is_ci() {
        args.must_get::<Value>("when")
    } else {
        args.must_get::<Value>("then")
    }
}

pub fn if_docker(args: Kwargs, _state: &State) -> TeraResult<Value> {
    if is_docker() {
        args.must_get::<Value>("when")
    } else {
        args.must_get::<Value>("then")
    }
}

// TASKS

// pub struct InputFunc {
//     project: Arc<Project>,
// }

// impl InputFunc {
//     pub fn new(project: Arc<Project>) -> Self {
//         Self { project }
//     }
// }

// impl Function<TeraResult<Value>> for InputFunc {
//     fn call(&self, args: Kwargs, state: &State) -> TeraResult<Value> {
//         let index = args.must_get::<u8>("index")?;
//         let inputs_raw = state.get::<serde_json::Value>("task_inputs")?;

//         let project = self
//             .graph
//             .get(id)
//             .map_err(|error| Error::message(error.to_string()))?;

//         let context = ProjectContext::new(&project);

//         Value::try_from_serializable(&context)
//     }
// }

// pub fn input(args: Kwargs, _state: &State) -> TeraResult<String> {
//     let index = args.must_get::<u8>("index")?;
//     let inputs_raw = args.must_get::<Value>("task_inputs")?;

//     match GlobalEnvBag::instance().get(key) {
//         Some(value) => Ok(value),
//         None => Ok(args.get::<&str>("fallback")?.unwrap_or_default().into()),
//     }
// }

// PROJECTS

pub struct GetProjectFunc {
    graph: Arc<ProjectGraph>,
}

impl GetProjectFunc {
    pub fn new(graph: Arc<ProjectGraph>) -> Self {
        Self { graph }
    }
}

impl Function<TeraResult<Value>> for GetProjectFunc {
    fn call(&self, args: Kwargs, _state: &State) -> TeraResult<Value> {
        let id = args.must_get::<&str>("id")?;

        let project = self
            .graph
            .get(id)
            .map_err(|error| Error::message(error.to_string()))?;

        let context = ProjectContext::new(&project);

        Value::try_from_serializable(&context)
    }
}
