use moon_app_context::AppContext;
use moon_emitter::{Event, EventFlow, Subscriber};
use moon_notifier::WebhooksNotifier;
use moon_utils::async_trait;

pub struct WebhooksSubscriber {
    notifier: WebhooksNotifier,
}

impl WebhooksSubscriber {
    pub fn new(notifier: WebhooksNotifier) -> Self {
        WebhooksSubscriber { notifier }
    }
}

#[async_trait]
impl Subscriber for WebhooksSubscriber {
    async fn on_emit<'e>(
        &mut self,
        event: &Event<'e>,
        _app_context: &AppContext,
    ) -> miette::Result<EventFlow> {
        self.notifier.notify(event.get_type(), event).await?;

        if event.is_end() {
            self.notifier.wait_for_requests().await;
        }

        Ok(EventFlow::Continue)
    }
}
