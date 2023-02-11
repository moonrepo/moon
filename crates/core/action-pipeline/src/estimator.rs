use moon_action::{Action, ActionNode};
use moon_target::Target;
use rustc_hash::FxHashMap;
use std::time::Duration;

struct Estimator {
    duration: Duration,
    tasks: FxHashMap<String, Duration>,
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

            match node {
                ActionNode::RunTarget(_, target) => {
                    let target = Target::parse(target).unwrap();

                    if let Some(overall_duration) = tasks.get_mut(&target.task_id) {
                        *overall_duration += *task_duration;
                    } else {
                        tasks.insert(target.task_id, task_duration.to_owned());
                    }
                }
                _ => {}
            }
        }

        // We assume every bucket is ran in parallel,
        // so use the longest/slowest bucket as the estimated duration.
        let duration = tasks.iter().fold(Duration::new(0, 0), |acc, task| {
            if &acc > task.1 {
                return acc;
            }

            return task.1.to_owned();
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
