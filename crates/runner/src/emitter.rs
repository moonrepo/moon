use crate::subscribers::local_cache::LocalCacheSubscriber;
use crate::ActionNode;
use moon_action::Action;
use moon_contract::{handle_flow, SupportedPlatform};
use moon_error::MoonError;
use moon_project::Project;
use moon_task::Task;
use moon_workspace::Workspace;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub use moon_contract::EventFlow;

#[derive(Debug)]
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
        platform: &'e SupportedPlatform,
        project_id: Option<&'e str>,
    },
    DependenciesInstalled {
        platform: &'e SupportedPlatform,
        project_id: Option<&'e str>,
    },

    // Syncing projects
    ProjectSyncing {
        platform: &'e SupportedPlatform,
        project_id: &'e str,
    },
    ProjectSynced {
        platform: &'e SupportedPlatform,
        project_id: &'e str,
    },

    // Runner
    RunAborted,
    RunStarted {
        actions_count: usize,
    },
    RunFinished {
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
        platform: &'e SupportedPlatform,
    },
    ToolInstalled {
        platform: &'e SupportedPlatform,
    },
}

pub struct RunnerEmitter {
    local_cache: Arc<RwLock<LocalCacheSubscriber>>,

    workspace: Arc<RwLock<Workspace>>,
}

impl RunnerEmitter {
    pub fn new(workspace: Arc<RwLock<Workspace>>) -> Self {
        RunnerEmitter {
            local_cache: Arc::new(RwLock::new(LocalCacheSubscriber::new())),
            workspace,
        }
    }

    pub async fn emit<'e>(&self, event: Event<'e>) -> Result<EventFlow, MoonError> {
        let workspace = self.workspace.read().await;

        // dbg!(&event);

        handle_flow!(
            self.local_cache
                .write()
                .await
                .on_emit(&event, &workspace)
                .await
        );

        Ok(EventFlow::Continue)
    }
}
