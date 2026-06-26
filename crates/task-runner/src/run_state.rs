use moon_action::Operation;
use moon_action_context::TargetState;
use moon_app_context::AppContext;
use moon_cache_item::cache_item;
use moon_hash::Digest;
use moon_remote::RemoteService;
use moon_task::Task;

cache_item!(
    pub struct TaskRunCacheState {
        pub exit_code: i32,
        pub hash: String,
        pub last_run_time: u128,
        pub target: String,
    }
);

#[derive(Default)]
pub struct TaskRunState {
    /// The digest of our internal fingerprint. This is separate from the action
    /// digest as this implementation is not Bazel compatible.
    pub digest: Digest,

    /// The last operation that was executed.
    pub operation: Operation,

    /// The final state of the target, for use within the action context.
    pub target: Option<TargetState>,

    /// Read and write states for the local/remote caches.
    pub local_cas_enabled: bool,
    pub local_cache_readable: bool,
    pub local_cache_writable: bool,
    pub remote_cache_readable: bool,
    pub remote_cache_writable: bool,
}

impl TaskRunState {
    pub fn new(app_context: &AppContext, task: &Task) -> Self {
        let remote_enabled =
            RemoteService::is_enabled() || app_context.cache_engine.storage.is_remote_enabled();

        Self {
            local_cas_enabled: app_context.workspace_config.experiments.cas_outputs_cache,
            local_cache_readable: app_context.cache_engine.is_readable()
                && task.options.cache.is_local_enabled(),
            local_cache_writable: app_context.cache_engine.is_writable()
                && task.options.cache.is_local_enabled(),
            remote_cache_readable: app_context.cache_engine.is_readable()
                && task.options.cache.is_remote_enabled()
                && remote_enabled,
            remote_cache_writable: app_context.cache_engine.is_writable()
                && task.options.cache.is_remote_enabled()
                && remote_enabled,
            ..Default::default()
        }
    }
}
