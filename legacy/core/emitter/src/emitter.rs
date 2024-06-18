use crate::event::{Event, EventFlow};
use crate::subscriber::Subscriber;
use moon_app_context::AppContext;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Emitter {
    pub subscribers: Arc<Mutex<Vec<Box<dyn Subscriber>>>>,

    app_context: Arc<AppContext>,
}

impl Emitter {
    pub fn new(app_context: Arc<AppContext>) -> Self {
        Emitter {
            subscribers: Arc::new(Mutex::new(vec![])),
            app_context,
        }
    }

    pub async fn subscribe(&self, subscriber: impl Subscriber + 'static) {
        let mut subscribers = self.subscribers.lock().await;

        subscribers.push(Box::new(subscriber));
    }

    pub async fn emit<'e>(&self, event: Event<'e>) -> miette::Result<EventFlow> {
        let mut subscribers = self.subscribers.lock().await;

        if !subscribers.is_empty() {
            for subscriber in subscribers.iter_mut() {
                match subscriber.on_emit(&event, &self.app_context).await? {
                    EventFlow::Break => return Ok(EventFlow::Break),
                    EventFlow::Return(value) => return Ok(EventFlow::Return(value)),
                    EventFlow::Continue => {}
                };
            }
        }

        Ok(EventFlow::Continue)
    }
}
