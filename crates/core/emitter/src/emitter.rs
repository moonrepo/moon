use crate::event::{Event, EventFlow};
use crate::subscriber::Subscriber;
use moon_error::MoonError;
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Emitter {
    pub subscribers: Vec<Arc<RwLock<dyn Subscriber>>>,

    workspace: Arc<RwLock<Workspace>>,
}

impl Emitter {
    pub fn new(workspace: Arc<RwLock<Workspace>>) -> Self {
        Emitter {
            subscribers: vec![],
            workspace,
        }
    }

    pub async fn emit<'e>(&self, event: Event<'e>) -> Result<EventFlow, MoonError> {
        let workspace = self.workspace.read().await;

        // dbg!(&event);

        for subscriber in &self.subscribers {
            match subscriber.write().await.on_emit(&event, &workspace).await? {
                EventFlow::Break => return Ok(EventFlow::Break),
                EventFlow::Return(value) => return Ok(EventFlow::Return(value)),
                EventFlow::Continue => {}
            };
        }

        Ok(EventFlow::Continue)
    }
}
