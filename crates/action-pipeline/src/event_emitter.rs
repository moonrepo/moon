use async_trait::async_trait;
use moon_action::{Action, ActionNode, ActionPipelineStatus, RunTaskNode};
use moon_action_context::ActionContext;
use moon_project::Project;
use moon_task::Target;
use moon_toolchain::Runtime;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::instrument;

#[derive(Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum Event<'data> {
    // Actions
    ActionStarted {
        action: &'data Action,
        node: &'data ActionNode,
    },
    ActionCompleted {
        action: &'data Action,
        error: Option<String>,
        #[serde(skip)]
        error_report: Option<&'data miette::Report>,
        node: &'data ActionNode,
    },

    // Installing deps
    DependenciesInstalling {
        project: Option<&'data Project>,
        runtime: &'data Runtime,
    },
    DependenciesInstalled {
        error: Option<String>,
        project: Option<&'data Project>,
        runtime: &'data Runtime,
    },

    #[serde(rename_all = "camelCase")]
    PipelineStarted {
        actions_count: usize,
        action_nodes: Vec<&'data ActionNode>,
        context: &'data ActionContext,
    },
    #[serde(rename_all = "camelCase")]
    PipelineCompleted {
        actions: &'data [Action],
        context: &'data ActionContext,
        duration: Option<Duration>,
        error: Option<String>,
        #[serde(skip)]
        error_report: Option<&'data miette::Report>,
        status: &'data ActionPipelineStatus,
    },

    // Syncing projects
    ProjectSyncing {
        project: &'data Project,
        runtime: &'data Runtime,
    },
    ProjectSynced {
        error: Option<String>,
        project: &'data Project,
        runtime: &'data Runtime,
    },

    // Running targets
    TaskRunning {
        node: &'data RunTaskNode,
        target: &'data Target,
    },
    TaskRan {
        error: Option<String>,
        node: &'data RunTaskNode,
        target: &'data Target,
    },

    // Installing a tool
    ToolInstalling {
        runtime: &'data Runtime,
    },
    ToolInstalled {
        error: Option<String>,
        runtime: &'data Runtime,
    },

    // Syncing workspace
    WorkspaceSyncing,
    WorkspaceSynced {
        error: Option<String>,
    },
}

impl Event<'_> {
    pub fn get_type(&self) -> &str {
        match self {
            Event::ActionStarted { .. } => "action.started",
            Event::ActionCompleted { .. } => "action.completed",
            Event::DependenciesInstalling { .. } => "dependencies.installing",
            Event::DependenciesInstalled { .. } => "dependencies.installed",
            Event::ProjectSyncing { .. } => "project.syncing",
            Event::ProjectSynced { .. } => "project.synced",
            Event::PipelineStarted { .. } => "pipeline.started",
            Event::PipelineCompleted { .. } => "pipeline.completed",
            Event::TaskRunning { .. } => "task.running",
            Event::TaskRan { .. } => "task.ran",
            Event::ToolInstalling { .. } => "tool.installing",
            Event::ToolInstalled { .. } => "tool.installed",
            Event::WorkspaceSyncing => "workspace.syncing",
            Event::WorkspaceSynced { .. } => "workspace.synced",
        }
    }
}

#[async_trait]
pub trait Subscriber: Send + Sync {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()>;
}

#[derive(Default)]
pub struct EventEmitter {
    subscribers: Arc<Mutex<Vec<Box<dyn Subscriber>>>>,
}

impl EventEmitter {
    pub async fn subscribe(&self, subscriber: impl Subscriber + 'static) {
        self.subscribers.lock().await.push(Box::new(subscriber));
    }

    #[instrument(skip_all)]
    pub async fn emit<'data>(&self, event: Event<'data>) -> miette::Result<()> {
        let mut subscribers = self.subscribers.lock().await;

        if !subscribers.is_empty() {
            for subscriber in subscribers.iter_mut() {
                subscriber.on_emit(&event).await?;
            }
        }

        Ok(())
    }
}
