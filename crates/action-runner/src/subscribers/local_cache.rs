use crate::events::Event;
use async_trait::async_trait;
use moon_contract::{EventFlow, Subscriber};
use moon_error::MoonError;

struct LocalCacheSubscriber;

#[async_trait]
impl Subscriber<Event<'static>> for LocalCacheSubscriber {
    async fn on_emit<'a>(&mut self, event: &Event<'a>) -> Result<EventFlow, MoonError> {
        match event {
            Event::TargetOutputCheckCache(workspace, hash) => {
                if workspace.cache.is_hash_cached(&hash) {
                    return Ok(EventFlow::Return("local-cache".into()));
                }
            }
            _ => {}
        }

        Ok(EventFlow::Continue)
    }
}
