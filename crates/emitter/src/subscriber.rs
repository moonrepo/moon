use crate::event::{Event, EventFlow};
use moon_error::MoonError;
use moon_workspace::Workspace;

#[async_trait::async_trait]
pub trait Subscriber {
    async fn on_emit<'e>(
        &mut self,
        event: &Event<'e>,
        workspace: &Workspace,
    ) -> Result<EventFlow, MoonError>;
}
