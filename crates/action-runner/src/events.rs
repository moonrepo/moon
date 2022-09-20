use moon_action::Action;
use std::path::PathBuf;
use std::time::Duration;

pub enum RunnerEvent<'a> {
    WorkflowAborted,
    WorkflowFinished(&'a Duration),
    WorkflowStarted(usize),

    // Actions
    ActionFinished(&'a Action),
    ActionRetried(&'a Action),
    ActionStarted(&'a Action),

    // Targets
    TargetCached(
        &'a String, // hash
        &'a String, // target
    ),
    TargetCheckCache(
        &'a String, // hash
        &'a String, // target
    ),
    TargetNotCached(
        &'a String, // hash
        &'a String, // target
    ),
    TargetOutputArchived(
        &'a String, // hash
        PathBuf,
    ),
    TargetOutputHydrated(
        &'a String, // hash
    ),
}

pub struct Emitter;

impl Emitter {
    pub async fn emit<'a>(&self, event: RunnerEvent<'a>) {
        // Loop over listeners
    }
}
