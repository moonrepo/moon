use moon_emitter::{Event, EventFlow, Subscriber};
use moon_utils::async_trait;
use moon_workspace::Workspace;

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
        workspace: &Workspace,
    ) -> miette::Result<EventFlow> {
        // After the run has finished, clean any stale archives.
        if let Event::PipelineFinished { .. } = event {
            if workspace.config.runner.auto_clean_cache {
                workspace
                    .cache_engine
                    .clean_stale_cache(&workspace.config.runner.cache_lifetime, false)?;
            }
        }

        Ok(EventFlow::Continue)
    }
}
