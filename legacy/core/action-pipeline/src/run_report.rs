use crate::estimator::Estimator;
use moon_action::Action;
use moon_action_context::ActionContext;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport<'a> {
    pub actions: &'a Vec<Action>,

    pub context: &'a ActionContext,

    /// How long the pipeline took to execute all actions.
    pub duration: Duration,

    /// Estimates around how much time was saved using moon,
    /// compared to another product or baseline.
    pub comparison_estimate: Estimator,
}

impl<'a> RunReport<'a> {
    pub fn new(
        actions: &'a Vec<Action>,
        context: &'a ActionContext,
        duration: Duration,
        estimate: Estimator,
    ) -> Self {
        RunReport {
            actions,
            context,
            duration,
            comparison_estimate: estimate,
        }
    }
}
