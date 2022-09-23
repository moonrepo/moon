use crate::events::Event;
use moon_contract::EventFlow;
use moon_error::MoonError;
use moon_workspace::Workspace;

pub struct LocalCacheSubscriber {}

impl LocalCacheSubscriber {
    pub fn new() -> Self {
        LocalCacheSubscriber {}
    }

    pub async fn on_emit<'a>(
        &mut self,
        event: &Event<'a>,
        workspace: &Workspace,
    ) -> Result<EventFlow, MoonError> {
        match event {
            Event::TargetOutputCheckCache(hash) => {
                if workspace.cache.is_hash_cached(hash) {
                    return Ok(EventFlow::Return("local-cache".into()));
                }
            }
            Event::RunFinished { .. } => {
                workspace
                    .cache
                    .clean_stale_cache(&workspace.config.runner.cache_lifetime)
                    .await?;
            }
            _ => {}
        }

        Ok(EventFlow::Continue)
    }
}

// #[async_trait]
// impl Subscriber<Event> for LocalCacheSubscriber {
//     async fn on_emit(&mut self, event: &Event) -> Result<EventFlow, MoonError> {
//         match event {
//             Event::TargetOutputCheckCache(workspace, hash) => {
//                 if workspace.cache.is_hash_cached(&hash) {
//                     return Ok(EventFlow::Return("local-cache".into()));
//                 }
//             }
//             _ => {}
//         }

//         Ok(EventFlow::Continue)
//     }
// }
