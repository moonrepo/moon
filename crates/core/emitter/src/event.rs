use moon_action::{Action, ActionNode};
use moon_action_context::ActionContext;
use moon_cache::RunTargetState;
use moon_platform_runtime::Runtime;
use moon_project::Project;
use moon_target::Target;
use moon_task::Task;
use serde::Serialize;
use std::{path::PathBuf, time::Duration};

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
        target: &'e Target,
    },
    TargetRan {
        error: Option<String>,
        target: &'e Target,
    },
    TargetOutputArchiving {
        #[serde(skip)]
        cache: &'e RunTargetState,
        hash: &'e str,
        project: &'e Project,
        target: &'e Target,
        task: &'e Task,
    },
    #[serde(rename_all = "camelCase")]
    TargetOutputArchived {
        archive_path: PathBuf,
        hash: &'e str,
        project: &'e Project,
        target: &'e Target,
        task: &'e Task,
    },
    TargetOutputHydrating {
        #[serde(skip)]
        cache: &'e RunTargetState,
        hash: &'e str,
        project: &'e Project,
        target: &'e Target,
        task: &'e Task,
    },
    #[serde(rename_all = "camelCase")]
    TargetOutputHydrated {
        archive_path: PathBuf,
        hash: &'e str,
        project: &'e Project,
        target: &'e Target,
        task: &'e Task,
    },
    TargetOutputCacheCheck {
        hash: &'e str,
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
            Event::TargetOutputArchiving { .. } => "target-output.archiving",
            Event::TargetOutputArchived { .. } => "target-output.archived",
            Event::TargetOutputHydrating { .. } => "target-output.hydrating",
            Event::TargetOutputHydrated { .. } => "target-output.hydrated",
            Event::TargetOutputCacheCheck { .. } => "target-output.cache-check",
            Event::ToolInstalling { .. } => "tool.installing",
            Event::ToolInstalled { .. } => "tool.installed",
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
