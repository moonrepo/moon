use crate::subscribers::local_cache::LocalCacheSubscriber;
use crate::ActionNode;
use moon_action::Action;
use moon_contract::{handle_flow, EventFlow};
use moon_error::MoonError;
use moon_workspace::Workspace;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug)]
pub enum Event<'e> {
    ActionAborted {
        action: &'e Action,
    },
    ActionStarted {
        action: &'e Action,
    },
    ActionFinished {
        action: &'e Action,
        node: &'e ActionNode,
    },

    TargetOutputArchive,
    TargetOutputHydrate,
    TargetOutputCheckCache(&'e str),

    RunAborted,
    RunStarted {
        actions_count: usize,
    },
    RunFinished {
        duration: &'e Duration,
        cached_count: u16,
        failed_count: u16,
        passed_count: u16,
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

        dbg!(&event);

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
