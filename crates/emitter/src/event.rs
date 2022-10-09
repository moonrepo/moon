use moon_action::{Action, ActionNode};
use moon_contract::Runtime;
use moon_project::Project;
use moon_task::Task;
use serde::Serialize;
use std::{path::PathBuf, time::Duration};

#[derive(Serialize)]
pub enum Event<'e> {
    // Actions
    ActionStarted {
        action: &'e Action,
        node: &'e ActionNode,
    },
    ActionFinished {
        action: &'e Action,
        node: &'e ActionNode,
    },

    // Installing deps
    DependenciesInstalling {
        project_id: Option<&'e str>,
        runtime: &'e Runtime,
    },
    DependenciesInstalled {
        project_id: Option<&'e str>,
        runtime: &'e Runtime,
    },

    // Syncing projects
    ProjectSyncing {
        project_id: &'e str,
        runtime: &'e Runtime,
    },
    ProjectSynced {
        project_id: &'e str,
        runtime: &'e Runtime,
    },

    // Runner
    RunnerAborted {},
    RunnerStarted {
        actions_count: usize,
    },
    RunnerFinished {
        duration: &'e Duration,
        cached_count: usize,
        failed_count: usize,
        passed_count: usize,
    },

    // Running targets
    TargetRunning {
        target_id: &'e str,
    },
    TargetRan {
        target_id: &'e str,
    },
    TargetOutputArchiving {
        hash: &'e str,
        project: &'e Project,
        task: &'e Task,
    },
    TargetOutputArchived {
        archive_path: PathBuf,
        hash: &'e str,
        project: &'e Project,
        task: &'e Task,
    },
    TargetOutputHydrating {
        hash: &'e str,
        project: &'e Project,
        task: &'e Task,
    },
    TargetOutputHydrated {
        archive_path: PathBuf,
        hash: &'e str,
        project: &'e Project,
        task: &'e Task,
    },
    TargetOutputCacheCheck {
        hash: &'e str,
        task: &'e Task,
    },

    // Installing a tool
    ToolInstalling {
        runtime: &'e Runtime,
    },
    ToolInstalled {
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
            Event::RunnerAborted { .. } => "runner.aborted",
            Event::RunnerStarted { .. } => "runner.started",
            Event::RunnerFinished { .. } => "runner.finished",
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
}

pub enum EventFlow {
    Break,
    Continue,
    Return(String),
}

#[macro_export]
macro_rules! handle_flow {
    ($result:expr) => {
        match $result? {
            EventFlow::Break => return Ok(EventFlow::Break),
            EventFlow::Return(value) => return Ok(EventFlow::Return(value)),
            EventFlow::Continue => {}
        };
    };
}
