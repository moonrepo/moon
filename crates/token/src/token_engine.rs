use std::sync::Arc;

use crate::contexts::*;
use crate::filters::*;
use crate::funcs::*;
use moon_graph_utils::GraphExpanderContext;
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_task::Task;
use tera::{Context, Tera};

#[derive(PartialEq)]
pub enum TokenScope {
    Command,
    Script,
    Args,
    Env,
    Inputs,
    Outputs,
}

impl TokenScope {
    pub fn label(&self) -> String {
        match self {
            TokenScope::Command => "commands",
            TokenScope::Script => "scripts",
            TokenScope::Args => "args",
            TokenScope::Env => "env",
            TokenScope::Inputs => "inputs",
            TokenScope::Outputs => "outputs",
        }
        .into()
    }
}

pub struct TokenEngine<'graph> {
    engine: Tera,

    pub scope: TokenScope,
    pub context: &'graph GraphExpanderContext,
    pub project: &'graph Project,
    pub project_graph: &'graph ProjectGraph,
}

impl<'graph> TokenEngine<'graph> {
    pub fn new(
        context: &'graph GraphExpanderContext,
        project_graph: &'graph Arc<ProjectGraph>,
        project: &'graph Project,
    ) -> Self {
        let mut engine = Tera::new();
        engine.register_filter("camel_case", camel_case);
        engine.register_filter("kebab_case", kebab_case);
        engine.register_filter("snake_case", snake_case);
        engine.register_filter("pascal_case", pascal_case);
        engine.register_filter("title_case", title_case);

        engine.register_function("env", env);
        engine.register_function("if_ci", if_ci);
        engine.register_function("if_docker", if_docker);
        engine.register_function("project", GetProjectFunc::new(Arc::clone(project_graph)));

        let global_context = engine.global_context();
        global_context.insert("host", &HostContext::new());
        global_context.insert("datetime", &DatetimeContext::new());
        global_context.insert("workspace", &WorkspaceContext::new(context));
        global_context.insert("extensions", &ExtensionsContext::new(context));
        global_context.insert("toolchains", &ToolchainsContext::new(context));
        global_context.insert("vcs", &VcsContext::new(context));
        global_context.insert("project", &ProjectContext::new(project));

        Self {
            scope: TokenScope::Args,
            context,
            engine,
            project,
            project_graph,
        }
    }

    pub fn set_task(&mut self, task: &Task) {
        let global_context = self.engine.global_context();
        global_context.insert("task", &TaskContext::new(task));
        global_context.insert("task_inputs", &task.inputs);
        global_context.insert("task_outputs", &task.outputs);
    }

    pub fn render(&self, template: &str) -> Result<String, tera::Error> {
        self.engine.render_str(template, &Context::default(), false)
    }
}
