use moon_cache::get_cache_mode;
use moon_emitter::{Event, EventFlow, Subscriber};
use moon_error::MoonError;
use moon_utils::{async_trait, path};
use moon_workspace::Workspace;

/// The local cache subscriber is in charge of managing archives
/// (task output's archived as tarballs), by reading and writing them
/// to the `.moon/cache/{outputs,hashes}` directories.
///
/// This is the last subscriber amongst all subscribers, as local
/// cache is the last line of defense. However, other subscribers
/// will piggyback off of it, like remote cache.
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
    ) -> Result<EventFlow, MoonError> {
        match event {
            // Check to see if a build with the provided hash has been cached locally.
            // We only check for the archive, as the manifest is purely for local debugging!
            Event::TargetOutputCacheCheck { hash, .. } => {
                if get_cache_mode().is_readable()
                    && workspace.cache.get_hash_archive_path(hash).exists()
                {
                    return Ok(EventFlow::Return("local-cache".into()));
                }
            }

            // Archive the task's outputs into the local cache.
            Event::TargetOutputArchiving {
                cache,
                hash,
                project,
                task,
                ..
            } => {
                let archive_path = workspace.cache.get_hash_archive_path(hash);

                if cache.archive_outputs(
                    &archive_path,
                    &workspace.root,
                    &project.source,
                    &task.outputs,
                )? {
                    return Ok(EventFlow::Return(path::to_string(archive_path)?));
                }
            }

            // Hydrate the cached archive into the task's outputs.
            Event::TargetOutputHydrating {
                cache,
                hash,
                project,
                task,
                ..
            } => {
                let archive_path = workspace.cache.get_hash_archive_path(hash);

                if cache.hydrate_outputs(
                    &archive_path,
                    &workspace.root,
                    &project.source,
                    &task.outputs,
                )? {
                    return Ok(EventFlow::Return(path::to_string(archive_path)?));
                }
            }

            // After the run has finished, clean any stale archives.
            Event::PipelineFinished { .. } => {
                workspace
                    .cache
                    .clean_stale_cache(&workspace.config.runner.cache_lifetime)?;
            }
            _ => {}
        }

        Ok(EventFlow::Continue)
    }
}
