use async_trait::async_trait;
use moon_error::MoonError;
use std::sync::{Arc, RwLock};

pub enum EventFlow {
    Break,
    Continue,
}

#[async_trait]
pub trait Subscriber<T>: Send + Sync {
    async fn on_emit(&mut self, event: &T) -> Result<EventFlow, MoonError>;
}

pub struct Emitter<T> {
    subscribers: Vec<Arc<RwLock<dyn Subscriber<T>>>>,
}

impl<T> Emitter<T> {
    pub async fn emit(&mut self, event: T) -> Result<EventFlow, MoonError> {
        for subscriber in &mut self.subscribers {
            let mut sub = subscriber
                .write()
                .expect("Unable to acquire write lock for subscriber.");

            if let EventFlow::Break = sub.on_emit(&event).await? {
                return Ok(EventFlow::Break);
            }
        }

        Ok(EventFlow::Continue)
    }

    pub fn subscribe(&mut self, subscriber: impl Subscriber<T> + 'static) {
        self.subscribers.push(Arc::new(RwLock::new(subscriber)));
    }
}
