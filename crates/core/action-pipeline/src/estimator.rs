use moon_action::{Action, ActionNode, ActionStatus};
use moon_target::Target;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEstimate {
    pub count: usize,
    pub total: Duration,
}

impl TaskEstimate {
    pub fn new(total: Duration) -> Self {
        TaskEstimate { count: 1, total }
    }

    pub fn with_count(total: Duration, count: usize) -> Self {
        TaskEstimate { count, total }
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimator {
    /// How long the actions would have taken to execute outside of moon.
    pub duration: Duration,

    /// How much less time was spent using moon's pipeline, compared to the baseline.
    pub gain: Option<Duration>,

    /// How much more time was spent using moon's pipeline, compared to the baseline.
    pub loss: Option<Duration>,

    // Percentage of savings between the baseline and current duration.
    pub percent: f32,

    /// Longest duration of each task bucketed by name.
    pub tasks: FxHashMap<String, TaskEstimate>,
}

impl Estimator {
    pub fn calculate(results: &[Action], pipeline_duration: Duration) -> Self {
        let mut tasks: FxHashMap<String, TaskEstimate> = FxHashMap::default();
        let mut install_duration = Duration::new(0, 0);

        // Bucket every ran target based on task name,
        // and aggregate all tasks of the same name.
        for result in results {
            let Some(node) = &result.node else {
                continue;
            };

            let Some(duration) = &result.duration else {
                continue;
            };

            let mut task_duration = duration.to_owned();

            // Comparisons don't utilize the same caching mechanisms that moon does,
            // so we need to emulate a fake duration on cache hit by multiplying it.
            if matches!(
                result.status,
                ActionStatus::Cached | ActionStatus::CachedFromRemote
            ) {
                task_duration *= 10;
            }

            match node {
                ActionNode::SetupTool(_)
                | ActionNode::InstallDeps(_)
                | ActionNode::InstallProjectDeps(_, _) => {
                    install_duration += task_duration;
                }
                ActionNode::RunTarget(_, target) => {
                    let task_id = Target::parse(target).unwrap().task_id;

                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.count += 1;
                        task.total += task_duration;
                    } else {
                        tasks.insert(task_id, TaskEstimate::new(task_duration.to_owned()));
                    }
                }
                _ => {}
            }
        }

        // Add all buckets together and attempt to emulate some form of parallelism.
        let comparison_duration = tasks.iter().fold(Duration::new(0, 0), |acc, (_, task)| {
            if task.count == 0 || task.total.is_zero() {
                return acc + task.total;
            }

            // Parallelism is very difficult to do, so we're shaving off 15% of the total duration
            let millis = task.total.as_millis() as f64 * 0.85;
            let secs = Duration::from_millis(millis as u64);

            acc + secs
        }) + install_duration;

        // We assume every bucket is ran in parallel,
        // so use the longest/slowest bucket as the estimated duration.
        // let comparison_duration = tasks.iter().fold(Duration::new(0, 0), |acc, (_, task)| {
        //     if acc > task.total {
        //         acc
        //     } else {
        //         task.total.clone()
        //     }
        // }) + install_duration;

        // Add the install duration for debugging purposes.
        if !install_duration.is_zero() {
            tasks.insert(
                "*".into(),
                TaskEstimate {
                    count: 0,
                    total: install_duration,
                },
            );
        }

        // Calculate the potential time savings gained/lost by comparing
        // the pipeline duration and our estimated duration.
        let mut loss = None;
        let mut gain = None;
        let mut percent = 0.0;

        if pipeline_duration < comparison_duration {
            gain = Some(comparison_duration - pipeline_duration);
            percent =
                (gain.as_ref().unwrap().as_secs_f32() / comparison_duration.as_secs_f32()) * 100.0;
        }

        if pipeline_duration > comparison_duration {
            loss = Some(pipeline_duration - comparison_duration);
            percent =
                -((loss.as_ref().unwrap().as_secs_f32() / pipeline_duration.as_secs_f32()) * 100.0);
        }

        Estimator {
            duration: comparison_duration,
            loss,
            tasks,
            gain,
            percent,
        }
    }
}
