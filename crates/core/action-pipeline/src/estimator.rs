use moon_action::{Action, ActionNode};
use moon_target::Target;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
pub struct Estimator {
    /// How long the actions would have taken to execute outside of moon.
    duration: Duration,

    /// Longest duration of each task bucketed by name.
    tasks: FxHashMap<String, Duration>,

    /// How much time was saved using moon's pipeline.
    savings: Option<Duration>,
}

impl Estimator {
    pub fn calculate(results: &[Action], pipeline_duration: Duration) -> Self {
        let mut tasks = FxHashMap::default();

        // Bucket every ran target based on task name,
        // and aggregate all tasks of the same name.
        for result in results {
            let Some(node) = &result.node else {
                continue;
            };

            let Some(task_duration) = &result.duration else {
                continue;
            };

            if let ActionNode::RunTarget(_, target) = node {
                let task_id = Target::parse(target).unwrap().task_id;

                if let Some(overall_duration) = tasks.get_mut(&task_id) {
                    *overall_duration += *task_duration;
                } else {
                    tasks.insert(task_id, task_duration.to_owned());
                }
            }
        }

        // We assume every bucket is ran in parallel,
        // so use the longest/slowest bucket as the estimated duration.
        let duration = tasks.iter().fold(Duration::new(0, 0), |acc, task| {
            if &acc > task.1 {
                acc
            } else {
                task.1.to_owned()
            }
        });

        // Calculate the potential time savings by comparing
        // the pipeline duration and our estimated duration.
        let mut savings = None;

        if pipeline_duration < duration {
            // Avoid "overflow when subtracting durations"
            savings = Some(duration - pipeline_duration);
        }

        Estimator {
            duration,
            tasks,
            savings,
        }
    }
}
