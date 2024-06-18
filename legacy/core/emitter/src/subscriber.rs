use crate::event::{Event, EventFlow};
use moon_app_context::AppContext;

#[async_trait::async_trait]
pub trait Subscriber: Send + Sync {
    async fn on_emit<'e>(
        &mut self,
        event: &Event<'e>,
        app_context: &AppContext,
    ) -> miette::Result<EventFlow>;
}
