use async_trait::async_trait;
use moon_error::MoonError;
use std::sync::Arc;
use tokio::sync::RwLock;

pub enum EventFlow {
    Break,
    Continue,
    Return(String),
}

#[async_trait]
pub trait Subscriber<T>: Send + Sync {
    async fn on_emit(&mut self, event: &T) -> Result<EventFlow, MoonError>;
}

pub struct Emitter<T> {
    subscribers: Vec<Arc<RwLock<dyn Subscriber<T>>>>,
}

impl<T> Emitter<T> {
    pub fn new() -> Self {
        Emitter {
            subscribers: vec![],
        }
    }

    pub async fn emit(&mut self, event: &T) -> Result<EventFlow, MoonError> {
        for subscriber in &mut self.subscribers {
            let mut sub = subscriber.write().await;

            match sub.on_emit(&event).await? {
                EventFlow::Break => return Ok(EventFlow::Break),
                EventFlow::Return(value) => return Ok(EventFlow::Return(value)),
                EventFlow::Continue => {}
            }
        }

        Ok(EventFlow::Continue)
    }

    pub fn subscribe(&mut self, subscriber: impl Subscriber<T> + 'static) {
        self.subscribers.push(Arc::new(RwLock::new(subscriber)));
    }
}
