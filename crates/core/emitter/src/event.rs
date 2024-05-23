use moon_action::{Action, ActionNode};
use moon_action_context::ActionContext;
use moon_platform_runtime::Runtime;
use moon_project::Project;
use moon_target::Target;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum Event<'e> {
    // Actions
    ActionStarted {
        action: &'e Action,
        node: &'e ActionNode,
    },
    ActionFinished {
        action: &'e Action,
        error: Option<String>,
        node: &'e ActionNode,
    },

    // Installing deps
    DependenciesInstalling {
        project: Option<&'e Project>,
        runtime: &'e Runtime,
    },
    DependenciesInstalled {
        error: Option<String>,
        project: Option<&'e Project>,
        runtime: &'e Runtime,
    },

    // Syncing projects
    ProjectSyncing {
        project: &'e Project,
        runtime: &'e Runtime,
    },
    ProjectSynced {
        error: Option<String>,
        project: &'e Project,
        runtime: &'e Runtime,
    },

    // Syncing workspace
    WorkspaceSyncing,
    WorkspaceSynced {
        error: Option<String>,
    },

    // Runner
    PipelineAborted {
        error: String,
    },
    #[serde(rename_all = "camelCase")]
    PipelineStarted {
        actions_count: usize,
        context: &'e ActionContext,
    },
    #[serde(rename_all = "camelCase")]
    PipelineFinished {
        baseline_duration: &'e Duration,
        cached_count: usize,
        context: &'e ActionContext,
        duration: &'e Duration,
        estimated_savings: Option<&'e Duration>,
        failed_count: usize,
        passed_count: usize,
    },

    // Running targets
    TargetRunning {
        #[serde(skip)]
        action: &'e Action,
        target: &'e Target,
    },
    TargetRan {
        #[serde(skip)]
        action: &'e Action,
        error: Option<String>,
        target: &'e Target,
    },

    // Installing a tool
    ToolInstalling {
        runtime: &'e Runtime,
    },
    ToolInstalled {
        error: Option<String>,
        runtime: &'e Runtime,
    },
}

impl<'e> Event<'e> {
    pub fn get_type(&self) -> String {
        let key = match self {
            Event::ActionStarted { .. } => "action.started",
            Event::ActionFinished { .. } => "action.finished",
            Event::DependenciesInstalling { .. } => "dependencies.installing",
            Event::DependenciesInstalled { .. } => "dependencies.installed",
            Event::ProjectSyncing { .. } => "project.syncing",
            Event::ProjectSynced { .. } => "project.synced",
            Event::PipelineAborted { .. } => "pipeline.aborted",
            Event::PipelineStarted { .. } => "pipeline.started",
            Event::PipelineFinished { .. } => "pipeline.finished",
            Event::TargetRunning { .. } => "target.running",
            Event::TargetRan { .. } => "target.ran",
            Event::ToolInstalling { .. } => "tool.installing",
            Event::ToolInstalled { .. } => "tool.installed",
            Event::WorkspaceSyncing => "workspace.syncing",
            Event::WorkspaceSynced { .. } => "workspace.synced",
        };

        key.to_owned()
    }

    pub fn is_end(&self) -> bool {
        matches!(self, Event::PipelineAborted { .. })
            || matches!(self, Event::PipelineFinished { .. })
    }
}

pub enum EventFlow {
    Break,
    Continue,
    Return(String),
}
