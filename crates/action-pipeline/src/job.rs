use async_trait::async_trait;
use moon_action::ActionNode;
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use tokio::sync::OwnedSemaphorePermit;
// use serde::{Deserialize, Serialize};
use crate::job_context::JobContext;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Semaphore};
use tokio_util::sync::CancellationToken;

pub struct Job {
    pub node: ActionNode,

    /// Contexts of all the things.
    pub context: JobContext,
    pub app_context: Arc<AppContext>,
    pub action_context: Arc<ActionContext>,

    /// Maximum seconds to run before it's cancelled.
    pub timeout: Option<u64>,

    /// Semaphore permit allowing it to be dispatched.
    pub permit: OwnedSemaphorePermit,
}
