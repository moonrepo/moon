use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_action::ActionPipelineStatus;
use moon_remote::RemoteService;
use tracing::debug;

#[derive(Default)]
pub struct RemoteSubscriber;

#[async_trait]
impl Subscriber for RemoteSubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        if matches!(
            event,
            Event::PipelineCompleted {
                status: ActionPipelineStatus::Completed,
                ..
            }
        ) {
            if let Some(session) = RemoteService::session() {
                debug!("Waiting for in-flight remote service requests to finish");

                session.wait_for_requests().await;
            }
        }

        Ok(())
    }
}
