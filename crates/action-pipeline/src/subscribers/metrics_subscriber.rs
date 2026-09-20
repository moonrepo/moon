use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_action::{Action, ActionNode, Operation};
use opentelemetry::metrics::{Counter, Histogram, Meter};
use opentelemetry::{KeyValue, global};
use std::time::Duration;

const METER_NAME: &str = "moon";

/// Records OTLP metrics for every action and operation that ran within the
/// pipeline. Only registered when OTEL exporting has been enabled, as the
/// global meter provider is otherwise a no-op.
pub struct MetricsSubscriber {
    action_executions: Counter<u64>,
    action_duration: Histogram<f64>,
    operation_executions: Counter<u64>,
    operation_duration: Histogram<f64>,
}

impl MetricsSubscriber {
    pub fn new() -> Self {
        Self::from_meter(global::meter(METER_NAME))
    }

    fn from_meter(meter: Meter) -> Self {
        Self {
            action_executions: meter
                .u64_counter("moon.action.executions")
                .with_description("Number of actions executed within the pipeline")
                .build(),
            action_duration: meter
                .f64_histogram("moon.action.duration")
                .with_description("Duration of actions executed within the pipeline")
                .with_unit("ms")
                .build(),
            operation_executions: meter
                .u64_counter("moon.operation.executions")
                .with_description("Number of operations executed within an action")
                .build(),
            operation_duration: meter
                .f64_histogram("moon.operation.duration")
                .with_description("Duration of operations executed within an action")
                .with_unit("ms")
                .build(),
        }
    }

    fn record_action(&self, action: &Action) {
        let attrs = get_action_attrs(action);

        self.action_executions.add(1, &attrs);

        if let Some(duration) = &action.duration {
            self.action_duration.record(as_millis(duration), &attrs);
        }

        for operation in action.operations.iter() {
            self.record_operation(action, operation);
        }
    }

    fn record_operation(&self, action: &Action, operation: &Operation) {
        let attrs = get_operation_attrs(action, operation);

        self.operation_executions.add(1, &attrs);

        if let Some(duration) = &operation.duration {
            self.operation_duration.record(as_millis(duration), &attrs);
        }

        // Plugin driven operations are nested within their parent operation
        for nested_operation in &operation.operations {
            self.record_operation(action, nested_operation);
        }
    }
}

#[async_trait]
impl Subscriber for MetricsSubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        if let Event::ActionCompleted { action, .. } = event {
            self.record_action(action);
        }

        Ok(())
    }
}

fn as_millis(duration: &Duration) -> f64 {
    // Many operations complete in microseconds, so avoid truncating
    // them all down to a flat 0 milliseconds
    duration.as_secs_f64() * 1000.0
}

fn get_action_attrs(action: &Action) -> Vec<KeyValue> {
    let mut attrs = vec![
        KeyValue::new("action", action.get_prefix()),
        KeyValue::new("status", action.status.get_type()),
    ];

    // Identify _what_ the action operated on, which is the most
    // useful dimension to group and filter metrics by
    match &*action.node {
        ActionNode::InstallDependencies(inner) => {
            attrs.push(KeyValue::new("toolchain", inner.toolchain_id.to_string()));

            if let Some(project_id) = &inner.project_id {
                attrs.push(KeyValue::new("project", project_id.to_string()));
            }
        }
        ActionNode::RunTask(inner) => {
            attrs.push(KeyValue::new("target", inner.target.to_string()));
        }
        ActionNode::SetupEnvironment(inner) => {
            attrs.push(KeyValue::new("toolchain", inner.toolchain_id.to_string()));

            if let Some(project_id) = &inner.project_id {
                attrs.push(KeyValue::new("project", project_id.to_string()));
            }
        }
        ActionNode::SetupToolchain(inner) => {
            attrs.push(KeyValue::new("toolchain", inner.toolchain.id.to_string()));
        }
        ActionNode::SyncProject(inner) => {
            attrs.push(KeyValue::new("project", inner.project_id.to_string()));
        }
        ActionNode::None | ActionNode::SetupProto(_) | ActionNode::SyncWorkspace => {}
    };

    attrs
}

fn get_operation_attrs(action: &Action, operation: &Operation) -> Vec<KeyValue> {
    // Purposefully avoid the action's target/project/toolchain here, as
    // multiplying them by every operation would explode the cardinality.
    // Traces provide this level of detail instead!
    let mut attrs = vec![
        KeyValue::new("action", action.get_prefix()),
        KeyValue::new("operation", operation.meta.get_type()),
        KeyValue::new("status", operation.status.get_type()),
    ];

    if let Some(id) = &operation.id {
        attrs.push(KeyValue::new("id", id.to_string()));
    }

    if let Some(plugin) = &operation.plugin {
        attrs.push(KeyValue::new("plugin", plugin.to_string()));
    }

    attrs
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_action::{ActionStatus, RunTaskNode, SetupToolchainNode, SyncProjectNode};
    use moon_common::Id;
    use moon_config::UnresolvedVersionSpec;
    use moon_task::Target;
    use moon_toolchain::ToolchainSpec;
    use opentelemetry::metrics::MeterProvider;
    use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
    use opentelemetry_sdk::metrics::{InMemoryMetricExporter, SdkMeterProvider};

    fn create_action(node: ActionNode, status: ActionStatus) -> Action {
        let mut action = Action::new(node);
        action.start();
        action.finish(status);
        action
    }

    fn create_task_action(status: ActionStatus) -> Action {
        create_action(
            ActionNode::run_task(RunTaskNode::new(Target::parse("app:build").unwrap())),
            status,
        )
    }

    fn get_attrs<'attr>(attrs: impl IntoIterator<Item = &'attr KeyValue>) -> Vec<(String, String)> {
        attrs
            .into_iter()
            .map(|attr| (attr.key.to_string(), attr.value.to_string()))
            .collect()
    }

    mod attributes {
        use super::*;

        #[test]
        fn includes_the_target_for_tasks() {
            assert_eq!(
                get_attrs(&get_action_attrs(&create_task_action(
                    ActionStatus::CachedFromRemote
                ))),
                vec![
                    ("action".into(), "run-task".into()),
                    ("status".into(), "cached-from-remote".into()),
                    ("target".into(), "app:build".into()),
                ]
            );
        }

        #[test]
        fn includes_the_toolchain_for_setup() {
            assert_eq!(
                get_attrs(&get_action_attrs(&create_action(
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("node"),
                            UnresolvedVersionSpec::parse("20.0.0").unwrap()
                        ),
                    }),
                    ActionStatus::Passed
                ))),
                vec![
                    ("action".into(), "setup-toolchain".into()),
                    ("status".into(), "passed".into()),
                    ("toolchain".into(), "node".into()),
                ]
            );
        }

        #[test]
        fn includes_the_project_for_syncs() {
            assert_eq!(
                get_attrs(&get_action_attrs(&create_action(
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("app"),
                    }),
                    ActionStatus::Skipped
                ))),
                vec![
                    ("action".into(), "sync-project".into()),
                    ("status".into(), "skipped".into()),
                    ("project".into(), "app".into()),
                ]
            );
        }

        #[test]
        fn has_no_identifier_for_workspace_syncs() {
            assert_eq!(
                get_attrs(&get_action_attrs(&create_action(
                    ActionNode::sync_workspace(),
                    ActionStatus::Passed
                ))),
                vec![
                    ("action".into(), "sync-workspace".into()),
                    ("status".into(), "passed".into()),
                ]
            );
        }

        #[test]
        fn includes_the_operation_id_and_plugin() {
            let action = create_action(ActionNode::sync_workspace(), ActionStatus::Passed);

            let mut operation = Operation::sync_operation("syncConfigs").unwrap();
            operation.plugin = Some(Id::raw("typescript"));
            operation.finish(ActionStatus::Passed);

            assert_eq!(
                get_attrs(&get_operation_attrs(&action, &operation)),
                vec![
                    ("action".into(), "sync-workspace".into()),
                    ("operation".into(), "sync-operation".into()),
                    ("status".into(), "passed".into()),
                    ("id".into(), "syncConfigs".into()),
                    ("plugin".into(), "typescript".into()),
                ]
            );
        }

        #[test]
        fn omits_the_target_from_operations() {
            let action = create_task_action(ActionStatus::Passed);
            let mut operation = Operation::hash_generation();
            operation.finish(ActionStatus::Passed);

            assert_eq!(
                get_attrs(&get_operation_attrs(&action, &operation)),
                vec![
                    ("action".into(), "run-task".into()),
                    ("operation".into(), "hash-generation".into()),
                    ("status".into(), "passed".into()),
                ]
            );
        }
    }

    mod recording {
        use super::*;

        /// A data point as `(metric name, attributes, value)`.
        type DataPoint = (String, Vec<(String, String)>, u64);

        struct TestHarness {
            exporter: InMemoryMetricExporter,
            provider: SdkMeterProvider,
            subscriber: MetricsSubscriber,
        }

        impl TestHarness {
            fn new() -> Self {
                let exporter = InMemoryMetricExporter::default();
                let provider = SdkMeterProvider::builder()
                    .with_periodic_exporter(exporter.clone())
                    .build();
                let subscriber = MetricsSubscriber::from_meter(provider.meter(METER_NAME));

                Self {
                    exporter,
                    provider,
                    subscriber,
                }
            }

            /// Returns each recorded data point, where counters report
            /// their sum and histograms their count.
            fn flush(&self) -> Vec<DataPoint> {
                self.provider.force_flush().unwrap();

                let mut points = vec![];

                for resource_metric in self.exporter.get_finished_metrics().unwrap() {
                    for scope_metric in resource_metric.scope_metrics() {
                        for metric in scope_metric.metrics() {
                            let name = metric.name().to_owned();

                            match metric.data() {
                                AggregatedMetrics::U64(MetricData::Sum(sum)) => {
                                    for point in sum.data_points() {
                                        points.push((
                                            name.clone(),
                                            get_attrs(point.attributes()),
                                            point.value(),
                                        ));
                                    }
                                }
                                AggregatedMetrics::F64(MetricData::Histogram(histogram)) => {
                                    for point in histogram.data_points() {
                                        points.push((
                                            name.clone(),
                                            get_attrs(point.attributes()),
                                            point.count(),
                                        ));
                                    }
                                }
                                _ => {}
                            };
                        }
                    }
                }

                // Metrics accumulate across flushes, so start each one fresh
                self.exporter.reset();

                points.sort();
                points
            }
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
        async fn records_actions_and_their_operations() {
            let harness = TestHarness::new();
            let mut action = create_task_action(ActionStatus::Passed);

            let mut hash_op = Operation::hash_generation();
            hash_op.finish(ActionStatus::Passed);
            action.operations.push(hash_op);

            let mut exec_op = Operation::task_execution("noop");
            exec_op.finish(ActionStatus::Failed);
            action.operations.push(exec_op);

            harness.subscriber.record_action(&action);

            let task_attrs = vec![
                ("action".to_owned(), "run-task".to_owned()),
                ("status".to_owned(), "passed".to_owned()),
                ("target".to_owned(), "app:build".to_owned()),
            ];

            assert_eq!(
                harness.flush(),
                vec![
                    ("moon.action.duration".to_owned(), task_attrs.clone(), 1),
                    ("moon.action.executions".to_owned(), task_attrs, 1),
                    (
                        "moon.operation.duration".to_owned(),
                        vec![
                            ("action".to_owned(), "run-task".to_owned()),
                            ("operation".to_owned(), "hash-generation".to_owned()),
                            ("status".to_owned(), "passed".to_owned()),
                        ],
                        1
                    ),
                    (
                        "moon.operation.duration".to_owned(),
                        vec![
                            ("action".to_owned(), "run-task".to_owned()),
                            ("operation".to_owned(), "task-execution".to_owned()),
                            ("status".to_owned(), "failed".to_owned()),
                        ],
                        1
                    ),
                    (
                        "moon.operation.executions".to_owned(),
                        vec![
                            ("action".to_owned(), "run-task".to_owned()),
                            ("operation".to_owned(), "hash-generation".to_owned()),
                            ("status".to_owned(), "passed".to_owned()),
                        ],
                        1
                    ),
                    (
                        "moon.operation.executions".to_owned(),
                        vec![
                            ("action".to_owned(), "run-task".to_owned()),
                            ("operation".to_owned(), "task-execution".to_owned()),
                            ("status".to_owned(), "failed".to_owned()),
                        ],
                        1
                    ),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
        async fn records_nested_plugin_operations() {
            let harness = TestHarness::new();
            let mut action = create_action(ActionNode::sync_workspace(), ActionStatus::Passed);

            let mut operation = Operation::sync_operation("syncConfigs").unwrap();
            let mut nested_operation = Operation::sync_operation("syncTsconfig").unwrap();
            nested_operation.plugin = Some(Id::raw("typescript"));
            nested_operation.finish(ActionStatus::Passed);
            operation.operations.push(nested_operation);
            operation.finish(ActionStatus::Passed);
            action.operations.push(operation);

            let executions = harness
                .flush()
                .into_iter()
                .filter(|(name, _, _)| name == "moon.operation.executions")
                .collect::<Vec<_>>();

            assert!(executions.is_empty());

            harness.subscriber.record_action(&action);

            let executions = harness
                .flush()
                .into_iter()
                .filter(|(name, _, _)| name == "moon.operation.executions")
                .collect::<Vec<_>>();

            assert_eq!(executions.len(), 2);
            assert!(executions.iter().any(|(_, attrs, _)| {
                attrs.contains(&("plugin".to_owned(), "typescript".to_owned()))
            }));
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
        async fn counts_unfinished_operations_without_a_duration() {
            let harness = TestHarness::new();
            let mut action = create_task_action(ActionStatus::Passed);

            // Never finished, so no duration was captured
            action.operations.push(Operation::mutex_acquisition());

            harness.subscriber.record_action(&action);

            let points = harness.flush();
            let names = points
                .iter()
                .filter(|(_, attrs, _)| {
                    attrs.contains(&("operation".to_owned(), "mutex-acquisition".to_owned()))
                })
                .map(|(name, _, _)| name.as_str())
                .collect::<Vec<_>>();

            assert_eq!(names, vec!["moon.operation.executions"]);
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
        async fn records_on_action_completed_events() {
            let mut harness = TestHarness::new();
            let action = create_task_action(ActionStatus::Cached);
            let node = ActionNode::sync_workspace();

            harness
                .subscriber
                .on_emit(&Event::ActionStarted {
                    action: &action,
                    node: &node,
                })
                .await
                .unwrap();

            assert!(harness.flush().is_empty());

            harness
                .subscriber
                .on_emit(&Event::ActionCompleted {
                    action: &action,
                    error: None,
                    error_report: None,
                    node: &node,
                })
                .await
                .unwrap();

            assert_eq!(
                harness
                    .flush()
                    .into_iter()
                    .map(|(name, _, value)| (name, value))
                    .collect::<Vec<_>>(),
                vec![
                    ("moon.action.duration".to_owned(), 1),
                    ("moon.action.executions".to_owned(), 1),
                ]
            );
        }

        #[test]
        fn uses_millisecond_durations() {
            assert_eq!(as_millis(&Duration::from_micros(1500)), 1.5);
            assert_eq!(as_millis(&Duration::from_secs(2)), 2000.0);
        }
    }
}
