use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry::metrics::Histogram;
use std::time::Duration;

pub(crate) struct ActionPipelineMetrics {
    task_concurrency_wait_duration: Histogram<u64>,
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

impl ActionPipelineMetrics {
    fn new() -> Self {
        let meter = global::meter("moon_action_pipeline");

        Self {
            task_concurrency_wait_duration: meter
                .u64_histogram("moon.task.concurrency_wait.duration")
                .with_description("Moon task concurrency wait duration.")
                .with_unit("ms")
                .build(),
        }
    }

    pub fn record_task_concurrency_wait(&self, task_target: &str, duration: Duration) {
        let attrs = [KeyValue::new("task_target", task_target.to_owned())];

        self.task_concurrency_wait_duration
            .record(duration_ms(duration), &attrs);
    }
}

pub(crate) fn action_pipeline_metrics() -> ActionPipelineMetrics {
    ActionPipelineMetrics::new()
}
