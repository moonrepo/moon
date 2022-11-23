use moon_action::Action;
use moon_runner_context::RunnerContext;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport<'a> {
    pub actions: &'a Vec<Action>,

    pub context: &'a RunnerContext,

    /// How long the runner took to execute all actions.
    pub duration: Duration,

    /// How much time was saved using the runner.
    pub estimated_savings: Option<Duration>,

    /// How long the actions would have taken to execute outside of the runner.
    pub projected_duration: Duration,
}

impl<'a> RunReport<'a> {
    pub fn new(actions: &'a Vec<Action>, context: &'a RunnerContext, duration: Duration) -> Self {
        let mut projected_duration = Duration::new(0, 0);

        for action in actions {
            if let Some(action_duration) = action.duration {
                projected_duration += action_duration;
            }
        }

        let mut estimated_savings = None;

        // Avoid "overflow when subtracting durations"
        if duration < projected_duration {
            estimated_savings = Some(projected_duration - duration);
        }

        RunReport {
            actions,
            context,
            duration,
            estimated_savings,
            projected_duration,
        }
    }
}
