use moon_action::ActionStatus;
use moon_common::is_ci_env;
use moon_task::Task;
use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry::metrics::{Counter, Histogram};
use std::time::Duration;

use crate::labels::action_status_label;

pub(crate) struct TaskRunnerMetrics {
    task_run_total: Counter<u64>,
    task_run_failure_total: Counter<u64>,
    task_retry_total: Counter<u64>,
    task_run_duration: Histogram<u64>,
    task_hash_generation_duration: Histogram<u64>,
    task_cache_lookup_total: Counter<u64>,
    task_cache_hit_total: Counter<u64>,
    task_cache_lookup_duration: Histogram<u64>,
    task_hydration_duration: Histogram<u64>,
    task_archive_duration: Histogram<u64>,
    task_execution_duration: Histogram<u64>,
    task_execution_attempts: Histogram<u64>,
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn build_base_attrs(task: &Task) -> Vec<KeyValue> {
    vec![
        KeyValue::new("task_target", task.target.to_string()),
        KeyValue::new("task_id", task.id.to_string()),
        KeyValue::new("task_type", task.type_of.to_string()),
        KeyValue::new("cache_enabled", task.options.cache.is_enabled()),
        KeyValue::new("ci", is_ci_env()),
    ]
}

impl TaskRunnerMetrics {
    fn new() -> Self {
        let meter = global::meter("moon_task_runner");

        Self {
            task_run_total: meter
                .u64_counter("moon.task.run.total")
                .with_description("Total number of moon task runs.")
                .build(),
            task_run_failure_total: meter
                .u64_counter("moon.task.run.failure.total")
                .with_description("Total number of failed moon task runs.")
                .build(),
            task_retry_total: meter
                .u64_counter("moon.task.retry.total")
                .with_description("Total number of moon task execution retries.")
                .build(),
            task_run_duration: meter
                .u64_histogram("moon.task.run.duration")
                .with_description("Overall moon task run duration.")
                .with_unit("ms")
                .build(),
            task_hash_generation_duration: meter
                .u64_histogram("moon.task.hash_generation.duration")
                .with_description("Moon task hash generation duration.")
                .with_unit("ms")
                .build(),
            task_cache_lookup_total: meter
                .u64_counter("moon.task.cache.lookup.total")
                .with_description("Total number of moon task cache lookups.")
                .build(),
            task_cache_hit_total: meter
                .u64_counter("moon.task.cache.hit.total")
                .with_description("Total number of moon task cache hits.")
                .build(),
            task_cache_lookup_duration: meter
                .u64_histogram("moon.task.cache.lookup.duration")
                .with_description("Moon task cache lookup duration.")
                .with_unit("ms")
                .build(),
            task_hydration_duration: meter
                .u64_histogram("moon.task.output.hydration.duration")
                .with_description("Moon task output hydration duration.")
                .with_unit("ms")
                .build(),
            task_archive_duration: meter
                .u64_histogram("moon.task.output.archive.duration")
                .with_description("Moon task output archive duration.")
                .with_unit("ms")
                .build(),
            task_execution_duration: meter
                .u64_histogram("moon.task.execution.duration")
                .with_description("Moon task command execution duration.")
                .with_unit("ms")
                .build(),
            task_execution_attempts: meter
                .u64_histogram("moon.task.execution.attempts")
                .with_description("Moon task execution attempt count.")
                .build(),
        }
    }

    pub fn record_task_run(
        &self,
        task: &Task,
        status: ActionStatus,
        cache_source: &str,
        interactive: bool,
        persistent: bool,
        duration: Duration,
    ) {
        let mut attrs = build_base_attrs(task);
        attrs.extend([
            KeyValue::new("status", action_status_label(status)),
            KeyValue::new("cache_source", cache_source.to_owned()),
            KeyValue::new("interactive", interactive),
            KeyValue::new("persistent", persistent),
        ]);

        self.task_run_total.add(1, &attrs);
        self.task_run_duration.record(duration_ms(duration), &attrs);

        if matches!(
            status,
            ActionStatus::Aborted
                | ActionStatus::Failed
                | ActionStatus::Invalid
                | ActionStatus::TimedOut
        ) {
            self.task_run_failure_total.add(1, &attrs);
        }
    }

    pub fn record_hash_generation(&self, task: &Task, duration: Duration) {
        let attrs = build_base_attrs(task);
        self.task_hash_generation_duration
            .record(duration_ms(duration), &attrs);
    }

    pub fn record_cache_lookup(&self, task: &Task, source: &str, hit: bool, duration: Duration) {
        let mut attrs = build_base_attrs(task);
        attrs.extend([
            KeyValue::new("cache_source", source.to_owned()),
            KeyValue::new("cache_hit", hit),
        ]);

        self.task_cache_lookup_total.add(1, &attrs);
        self.task_cache_lookup_duration
            .record(duration_ms(duration), &attrs);

        if hit {
            self.task_cache_hit_total.add(1, &attrs);
        }
    }

    pub fn record_hydration(
        &self,
        task: &Task,
        hydrate_from: &str,
        status: ActionStatus,
        hydrated: bool,
        duration: Duration,
    ) {
        let mut attrs = build_base_attrs(task);
        attrs.extend([
            KeyValue::new("hydrate_from", hydrate_from.to_owned()),
            KeyValue::new("hydrated", hydrated),
            KeyValue::new("status", action_status_label(status)),
        ]);

        self.task_hydration_duration
            .record(duration_ms(duration), &attrs);
    }

    pub fn record_archive(
        &self,
        task: &Task,
        archived: bool,
        status: ActionStatus,
        duration: Duration,
    ) {
        let mut attrs = build_base_attrs(task);
        attrs.extend([
            KeyValue::new("archived", archived),
            KeyValue::new("status", action_status_label(status)),
        ]);

        self.task_archive_duration
            .record(duration_ms(duration), &attrs);
    }

    pub fn record_execution(
        &self,
        task: &Task,
        status: ActionStatus,
        interactive: bool,
        persistent: bool,
        duration: Duration,
    ) {
        let mut attrs = build_base_attrs(task);
        attrs.extend([
            KeyValue::new("status", action_status_label(status)),
            KeyValue::new("interactive", interactive),
            KeyValue::new("persistent", persistent),
        ]);

        self.task_execution_duration
            .record(duration_ms(duration), &attrs);
    }

    pub fn record_execution_attempts(
        &self,
        task: &Task,
        status: ActionStatus,
        interactive: bool,
        persistent: bool,
        attempts: u64,
    ) {
        let mut attrs = build_base_attrs(task);
        attrs.extend([
            KeyValue::new("status", action_status_label(status)),
            KeyValue::new("interactive", interactive),
            KeyValue::new("persistent", persistent),
        ]);

        self.task_execution_attempts.record(attempts, &attrs);

        if attempts > 1 {
            self.task_retry_total.add(attempts - 1, &attrs);
        }
    }
}

pub(crate) fn task_runner_metrics() -> TaskRunnerMetrics {
    TaskRunnerMetrics::new()
}
