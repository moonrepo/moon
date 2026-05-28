use moon_action::ActionStatus;

pub(crate) fn action_status_label(status: ActionStatus) -> &'static str {
    match status {
        ActionStatus::Aborted => "aborted",
        ActionStatus::Cached => "cached",
        ActionStatus::CachedFromRemote => "cached-from-remote",
        ActionStatus::Failed => "failed",
        ActionStatus::Invalid => "invalid",
        ActionStatus::Passed => "passed",
        ActionStatus::Running => "running",
        ActionStatus::Skipped => "skipped",
        ActionStatus::TimedOut => "timed-out",
    }
}
