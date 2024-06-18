use moon_app_context::AppContext;
use moon_emitter::{Event, EventFlow, Subscriber};
use moon_utils::async_trait;

pub struct LocalCacheSubscriber {}

impl LocalCacheSubscriber {
    pub fn new() -> Self {
        LocalCacheSubscriber {}
    }
}

#[async_trait]
impl Subscriber for LocalCacheSubscriber {
    async fn on_emit<'e>(
        &mut self,
        event: &Event<'e>,
        app_context: &AppContext,
    ) -> miette::Result<EventFlow> {
        // After the run has finished, clean any stale archives.
        if let Event::PipelineFinished { .. } = event {
            if app_context.workspace_config.runner.auto_clean_cache {
                app_context.cache_engine.clean_stale_cache(
                    &app_context.workspace_config.runner.cache_lifetime,
                    false,
                )?;
            }
        }

        Ok(EventFlow::Continue)
    }
}
