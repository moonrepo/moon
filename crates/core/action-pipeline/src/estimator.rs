use moon_action::{Action, ActionNode};
use moon_target::Target;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimator {
    /// How long the actions would have taken to execute outside of moon.
    pub duration: Duration,

    /// How much more time was spent using moon's pipeline, compared to the baseline.
    pub loss: Option<Duration>,

    /// Longest duration of each task bucketed by name.
    pub tasks: FxHashMap<String, Duration>,

    /// How much less time was spent using moon's pipeline, compared to the baseline.
    pub gain: Option<Duration>,

    // Percentage of savings between the baseline and current duration.
    pub percent: f32,
}

impl Estimator {
    pub fn calculate(results: &[Action], pipeline_duration: Duration) -> Self {
        let mut tasks = FxHashMap::default();
        let mut install_duration = Duration::new(0, 0);

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
                ActionNode::SetupTool(_)
                | ActionNode::InstallDeps(_)
                | ActionNode::InstallProjectDeps(_, _) => {
                    install_duration += *task_duration;
                }
                ActionNode::RunTarget(_, target) => {
                    let task_id = Target::parse(target).unwrap().task_id;

                    if let Some(overall_duration) = tasks.get_mut(&task_id) {
                        *overall_duration += *task_duration;
                    } else {
                        tasks.insert(task_id, task_duration.to_owned());
                    }
                }
                _ => {}
            }
        }

        // We assume every bucket is ran in parallel,
        // so use the longest/slowest bucket as the estimated duration.
        let comparison_duration = tasks.iter().fold(Duration::new(0, 0), |acc, task| {
            if &acc > task.1 {
                acc
            } else {
                task.1.to_owned()
            }
        }) + install_duration;

        // Add the install duration for debugging purposes.
        if !install_duration.is_zero() {
            tasks.insert("*".into(), install_duration);
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
        } else if pipeline_duration > comparison_duration {
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
