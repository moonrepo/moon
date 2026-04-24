mod utils;

use moon_action_context::ActionContext;
use opentelemetry_proto::tonic::collector::metrics::v1::{
    ExportMetricsServiceRequest, ExportMetricsServiceResponse,
    metrics_service_server::{MetricsService, MetricsServiceServer},
};
use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTraceServiceRequest, ExportTraceServiceResponse,
    trace_service_server::{TraceService, TraceServiceServer},
};
use starbase::tracing::{OtelOptions, TracingOptions, setup_tracing};
use std::env;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status, transport::Server};
use utils::TaskRunnerContainer;

fn string_attr<'a>(
    span: &'a opentelemetry_proto::tonic::trace::v1::Span,
    key: &str,
) -> Option<&'a str> {
    span.attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .and_then(|attribute| attribute.value.as_ref())
        .and_then(|value| value.value.as_ref())
        .and_then(|value| match value {
            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value) => {
                Some(value.as_str())
            }
            _ => None,
        })
}

fn bool_attr(span: &opentelemetry_proto::tonic::trace::v1::Span, key: &str) -> Option<bool> {
    span.attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .and_then(|attribute| attribute.value.as_ref())
        .and_then(|value| value.value.as_ref())
        .and_then(|value| match value {
            opentelemetry_proto::tonic::common::v1::any_value::Value::BoolValue(value) => {
                Some(*value)
            }
            _ => None,
        })
}

fn any_value_string(value: &opentelemetry_proto::tonic::common::v1::AnyValue) -> Option<&str> {
    value.value.as_ref().and_then(|value| match value {
        opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value) => {
            Some(value.as_str())
        }
        _ => None,
    })
}

fn any_value_bool(value: &opentelemetry_proto::tonic::common::v1::AnyValue) -> Option<bool> {
    value.value.as_ref().and_then(|value| match value {
        opentelemetry_proto::tonic::common::v1::any_value::Value::BoolValue(value) => Some(*value),
        _ => None,
    })
}

fn key_value_matches(
    attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue],
    key: &str,
    expected: &str,
) -> bool {
    attributes.iter().any(|attribute| {
        attribute.key == key
            && attribute
                .value
                .as_ref()
                .and_then(any_value_string)
                .is_some_and(|value| value == expected)
    })
}

fn key_value_bool_matches(
    attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue],
    key: &str,
    expected: bool,
) -> bool {
    attributes.iter().any(|attribute| {
        attribute.key == key
            && attribute
                .value
                .as_ref()
                .and_then(any_value_bool)
                .is_some_and(|value| value == expected)
    })
}

fn metric_has_string_attr(
    metric: &opentelemetry_proto::tonic::metrics::v1::Metric,
    key: &str,
    expected: &str,
) -> bool {
    match metric.data.as_ref() {
        Some(opentelemetry_proto::tonic::metrics::v1::metric::Data::Sum(sum)) => sum
            .data_points
            .iter()
            .any(|point| key_value_matches(&point.attributes, key, expected)),
        Some(opentelemetry_proto::tonic::metrics::v1::metric::Data::Histogram(histogram)) => {
            histogram
                .data_points
                .iter()
                .any(|point| key_value_matches(&point.attributes, key, expected))
        }
        _ => false,
    }
}

fn metric_has_string_and_bool_attrs(
    metric: &opentelemetry_proto::tonic::metrics::v1::Metric,
    string_key: &str,
    string_expected: &str,
    bool_key: &str,
    bool_expected: bool,
) -> bool {
    let point_matches = |attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue]| {
        key_value_matches(attributes, string_key, string_expected)
            && key_value_bool_matches(attributes, bool_key, bool_expected)
    };

    match metric.data.as_ref() {
        Some(opentelemetry_proto::tonic::metrics::v1::metric::Data::Sum(sum)) => sum
            .data_points
            .iter()
            .any(|point| point_matches(&point.attributes)),
        Some(opentelemetry_proto::tonic::metrics::v1::metric::Data::Histogram(histogram)) => {
            histogram
                .data_points
                .iter()
                .any(|point| point_matches(&point.attributes))
        }
        _ => false,
    }
}

#[derive(Clone, Default)]
struct Collector {
    metric_requests: Arc<Mutex<Vec<ExportMetricsServiceRequest>>>,
    trace_requests: Arc<Mutex<Vec<ExportTraceServiceRequest>>>,
}

#[tonic::async_trait]
impl MetricsService for Collector {
    async fn export(
        &self,
        request: Request<ExportMetricsServiceRequest>,
    ) -> Result<Response<ExportMetricsServiceResponse>, Status> {
        self.metric_requests.lock().await.push(request.into_inner());

        Ok(Response::new(ExportMetricsServiceResponse {
            partial_success: None,
        }))
    }
}

#[tonic::async_trait]
impl TraceService for Collector {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        self.trace_requests.lock().await.push(request.into_inner());

        Ok(Response::new(ExportTraceServiceResponse {
            partial_success: None,
        }))
    }
}

struct EnvVarGuard {
    key: &'static str,
    old_value: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: String) -> Self {
        let old_value = env::var(key).ok();

        unsafe {
            env::set_var(key, value);
        }

        Self { key, old_value }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.old_value {
            unsafe {
                env::set_var(self.key, value);
            }
        } else {
            unsafe {
                env::remove_var(self.key);
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exports_task_runner_spans_over_otlp() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let metric_requests = Arc::new(Mutex::new(Vec::new()));
    let trace_requests = Arc::new(Mutex::new(Vec::new()));
    let collector = Collector {
        metric_requests: Arc::clone(&metric_requests),
        trace_requests: Arc::clone(&trace_requests),
    };
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(
                MetricsServiceServer::new(collector.clone())
                    .max_decoding_message_size(32 * 1024 * 1024),
            )
            .add_service(
                TraceServiceServer::new(collector).max_decoding_message_size(32 * 1024 * 1024),
            )
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });

    let _endpoint = EnvVarGuard::set(
        "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
        format!("http://{addr}"),
    );
    let _protocol = EnvVarGuard::set("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL", "grpc".into());
    let _metrics_endpoint = EnvVarGuard::set(
        "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
        format!("http://{addr}"),
    );
    let _metrics_protocol = EnvVarGuard::set("OTEL_EXPORTER_OTLP_METRICS_PROTOCOL", "grpc".into());

    let guard = setup_tracing(TracingOptions {
        otel: OtelOptions {
            enabled: true,
            logs_enabled: false,
            service_name: Some("moon-task-runner-test".into()),
        },
        ..TracingOptions::default()
    });

    let container = TaskRunnerContainer::new_os("runner", "create-file").await;
    container.sandbox.enable_git();

    let mut runner = container.create_runner();
    let node = container.create_action_node();
    let context = ActionContext::default();

    runner.run_with_panic(&context, &node).await.unwrap();
    fs::remove_file(container.project.root.join("file.txt")).unwrap();

    let mut cached_runner = container.create_runner();
    cached_runner.run_with_panic(&context, &node).await.unwrap();
    drop(guard);

    let wait_result = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if !trace_requests.lock().await.is_empty() && !metric_requests.lock().await.is_empty() {
                break;
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    if wait_result.is_err() {
        let trace_count = trace_requests.lock().await.len();
        let metric_count = metric_requests.lock().await.len();

        panic!(
            "timed out waiting for task runner OTLP export (traces={trace_count}, metrics={metric_count})"
        );
    }

    let trace_requests = trace_requests.lock().await.clone();
    let metric_requests = metric_requests.lock().await.clone();
    let spans = trace_requests
        .iter()
        .flat_map(|request| request.resource_spans.iter())
        .flat_map(|resource| resource.scope_spans.iter())
        .flat_map(|scope| scope.spans.iter())
        .collect::<Vec<_>>();
    let metrics = metric_requests
        .iter()
        .flat_map(|request| request.resource_metrics.iter())
        .flat_map(|resource| resource.scope_metrics.iter())
        .flat_map(|scope| scope.metrics.iter())
        .collect::<Vec<_>>();

    let has_span = |name: &str| spans.iter().any(|span| span.name == name);
    let has_metric = |name: &str| metrics.iter().any(|metric| metric.name == name);
    let observed_span_names = spans
        .iter()
        .map(|span| span.name.as_str())
        .collect::<Vec<_>>();
    let observed_metric_names = metrics
        .iter()
        .map(|metric| metric.name.as_str())
        .collect::<Vec<_>>();
    let disallowed_attrs = [
        "args",
        "command_line",
        "hash",
        "node",
        "project",
        "root",
        "task_command",
    ];
    let reviewed_span_names = [
        "process_spawn",
        "task_cache_lookup",
        "task_execution",
        "task_hash_generation",
        "task_output_hydration",
        "task_run",
    ];
    let task_run_miss = spans
        .iter()
        .find(|span| span.name == "task_run" && string_attr(span, "cache_source") == Some("miss"));
    let task_run_local_cache = spans.iter().find(|span| {
        span.name == "task_run" && string_attr(span, "cache_source") == Some("local-cache")
    });

    assert!(
        has_span("task_run"),
        "observed spans: {observed_span_names:?}"
    );
    assert!(
        has_span("task_hash_generation"),
        "observed spans: {observed_span_names:?}"
    );
    assert!(
        has_span("task_cache_lookup"),
        "observed spans: {observed_span_names:?}"
    );
    assert!(
        has_span("task_output_hydration"),
        "observed spans: {observed_span_names:?}"
    );
    assert!(
        has_span("task_execution"),
        "observed spans: {observed_span_names:?}"
    );
    assert!(
        has_span("task_execution_attempts"),
        "observed spans: {observed_span_names:?}"
    );
    assert!(
        has_span("process_spawn"),
        "observed spans: {observed_span_names:?}"
    );
    assert!(
        has_span("task_output_archive"),
        "observed spans: {observed_span_names:?}"
    );
    assert!(
        has_metric("moon.task.run.total"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.run.duration"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.hash_generation.duration"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.cache.lookup.total"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.cache.lookup.duration"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.output.hydration.duration"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.execution.duration"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.execution.attempts"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.output.archive.duration"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert_eq!(
        task_run_miss.and_then(|span| string_attr(span, "task_target")),
        Some(container.task.target.as_str())
    );
    assert!(
        task_run_local_cache.is_some(),
        "expected task_run span with cache_source=local-cache; observed spans: {observed_span_names:?}"
    );
    let cache_lookup_local_cache = spans.iter().find(|span| {
        span.name == "task_cache_lookup" && string_attr(span, "cache_source") == Some("local-cache")
    });
    assert_eq!(
        cache_lookup_local_cache.and_then(|span| bool_attr(span, "cache_hit")),
        Some(true),
        "expected task_cache_lookup span to include cache_source=local-cache and cache_hit=true; observed spans: {observed_span_names:?}"
    );
    let offending_attrs = spans
        .iter()
        .filter(|span| reviewed_span_names.contains(&span.name.as_str()))
        .flat_map(|span| {
            span.attributes.iter().filter_map(|attribute| {
                disallowed_attrs
                    .contains(&attribute.key.as_str())
                    .then(|| format!("{}:{}", span.name, attribute.key))
            })
        })
        .collect::<Vec<_>>();

    assert!(
        offending_attrs.is_empty(),
        "exported spans unexpectedly included high-cardinality attrs: {offending_attrs:?}; observed spans: {observed_span_names:?}"
    );
    assert!(
        metrics.iter().any(|metric| {
            metric.name == "moon.task.cache.hit.total"
                && metric_has_string_attr(metric, "cache_source", "local-cache")
        }),
        "expected moon.task.cache.hit.total to include cache_source=local-cache; observed metrics: {observed_metric_names:?}"
    );
    assert!(
        metrics.iter().any(|metric| {
            metric.name == "moon.task.cache.lookup.total"
                && metric_has_string_and_bool_attrs(
                    metric,
                    "cache_source",
                    "local-cache",
                    "cache_hit",
                    true,
                )
        }),
        "expected moon.task.cache.lookup.total to include cache_source=local-cache and cache_hit=true; observed metrics: {observed_metric_names:?}"
    );
    assert!(
        metrics.iter().any(|metric| {
            metric.name == "moon.task.output.hydration.duration"
                && metric_has_string_attr(metric, "hydrate_from", "local-cache")
        }),
        "expected moon.task.output.hydration.duration to include hydrate_from=local-cache; observed metrics: {observed_metric_names:?}"
    );

    let _ = shutdown_tx.send(());
    server.await.unwrap();
}
