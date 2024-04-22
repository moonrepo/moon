use crate::event::{Event, EventFlow};
use crate::subscriber::Subscriber;
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Emitter {
    pub subscribers: Vec<Arc<RwLock<dyn Subscriber>>>,

    workspace: Arc<Workspace>,
}

impl Emitter {
    pub fn new(workspace: Arc<Workspace>) -> Self {
        Emitter {
            subscribers: vec![],
            workspace,
        }
    }

    pub async fn emit<'e>(&self, event: Event<'e>) -> miette::Result<EventFlow> {
        // dbg!(&event);

        for subscriber in &self.subscribers {
            match subscriber
                .write()
                .await
                .on_emit(&event, &self.workspace)
                .await?
            {
                EventFlow::Break => return Ok(EventFlow::Break),
                EventFlow::Return(value) => return Ok(EventFlow::Return(value)),
                EventFlow::Continue => {}
            };
        }

        Ok(EventFlow::Continue)
    }
}
