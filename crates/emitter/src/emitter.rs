use crate::event::{Event, EventFlow};
use crate::handle_flow;
use crate::subscriber::Subscriber;
use moon_error::MoonError;
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Emitter {
    subscribers: Vec<Arc<RwLock<dyn Subscriber>>>,

    workspace: Arc<RwLock<Workspace>>,
}

impl Emitter {
    pub async fn new(
        workspace: Arc<RwLock<Workspace>>,
        subscribers: Vec<Arc<RwLock<dyn Subscriber>>>,
    ) -> Self {
        Emitter {
            subscribers,
            workspace,
        }
    }

    pub async fn emit<'e>(&self, event: Event<'e>) -> Result<EventFlow, MoonError> {
        let workspace = self.workspace.read().await;

        // dbg!(&event);

        for subscriber in &self.subscribers {
            handle_flow!(subscriber.write().await.on_emit(&event, &workspace).await);
        }

        Ok(EventFlow::Continue)
    }
}
