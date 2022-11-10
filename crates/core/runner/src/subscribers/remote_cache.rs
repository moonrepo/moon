#![allow(dead_code)]
#![allow(unused_variables)]

use moon_emitter::{Event, EventFlow, Subscriber};
use moon_error::MoonError;
use moon_workspace::Workspace;
use moonbase::MoonbaseError;

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
        let Some(moonbase) = &workspace.session else {
            return Ok(EventFlow::Continue);
        };

        let error_handler = |e: MoonbaseError| MoonError::Generic(e.to_string());

        match event {
            // Check if archive exists in moonbase (the remote) by quering the artifacts endpoint.
            Event::TargetOutputCacheCheck { hash, .. } => {
                if moonbase
                    .get_artifact(hash)
                    .await
                    .map_err(error_handler)?
                    .is_some()
                {
                    return Ok(EventFlow::Return("remote-cache".into()));
                }
            }

            // The local cache subscriber uses the `TargetOutputArchiving` event to create
            // the tarball. This runs *after* it's been created so that we can upload it.
            Event::TargetOutputArchived {
                archive_path,
                hash,
                project,
                target,
                task,
            } => {
                if archive_path.exists() {
                    moonbase
                        .upload_artifact(hash, target, archive_path)
                        .await
                        .map_err(error_handler)?;
                }
            }

            // Hydrate the cached archive into the task's outputs
            Event::TargetOutputHydrating { hash, project, .. } => {}

            _ => {}
        }

        Ok(EventFlow::Continue)
    }
}
