#![allow(dead_code)]
#![allow(unused_variables)]

use moon_emitter::{Event, EventFlow, Subscriber};
use moon_error::MoonError;
use moon_workspace::Workspace;

pub struct RemoteCacheSubscriber {}

impl RemoteCacheSubscriber {
    pub fn new() -> Self {
        RemoteCacheSubscriber {}
    }
}

#[async_trait::async_trait]
impl Subscriber for RemoteCacheSubscriber {
    async fn on_emit<'a>(
        &mut self,
        event: &Event<'a>,
        workspace: &Workspace,
    ) -> Result<EventFlow, MoonError> {
        match event {
            // Check if archive exists in moonbase
            Event::TargetOutputCacheCheck { hash, .. } => {
                // if workspace.cache.is_hash_cached(hash) {
                //     return Ok(EventFlow::Return("remote-cache".into()));
                // }
            }

            // Update the archive to moonbase
            Event::TargetOutputArchived {
                archive_path,
                hash,
                project,
                task,
            } => {}

            // Hydrate the cached archive into the task's outputs
            Event::TargetOutputHydrating { hash, project, .. } => {}

            _ => {}
        }

        Ok(EventFlow::Continue)
    }
}
