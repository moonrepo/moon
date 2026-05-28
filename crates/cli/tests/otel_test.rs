use moon_test_utils2::{create_empty_moon_sandbox, predicates::prelude::*};
use opentelemetry_proto::tonic::collector::logs::v1::{
    ExportLogsServiceRequest, ExportLogsServiceResponse,
    logs_service_server::{LogsService, LogsServiceServer},
};
use opentelemetry_proto::tonic::collector::metrics::v1::{
    ExportMetricsServiceRequest, ExportMetricsServiceResponse,
    metrics_service_server::{MetricsService, MetricsServiceServer},
};
use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTraceServiceRequest, ExportTraceServiceResponse,
    trace_service_server::{TraceService, TraceServiceServer},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status, transport::Server};

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
    log_requests: Arc<Mutex<Vec<ExportLogsServiceRequest>>>,
    metric_requests: Arc<Mutex<Vec<ExportMetricsServiceRequest>>>,
    trace_requests: Arc<Mutex<Vec<ExportTraceServiceRequest>>>,
}

#[tonic::async_trait]
impl LogsService for Collector {
    async fn export(
        &self,
        request: Request<ExportLogsServiceRequest>,
    ) -> Result<Response<ExportLogsServiceResponse>, Status> {
        self.log_requests.lock().await.push(request.into_inner());

        Ok(Response::new(ExportLogsServiceResponse {
            partial_success: None,
        }))
    }
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

async fn wait_for_cli_exports(
    trace_requests: &Arc<Mutex<Vec<ExportTraceServiceRequest>>>,
    metric_requests: &Arc<Mutex<Vec<ExportMetricsServiceRequest>>>,
    log_requests: &Arc<Mutex<Vec<ExportLogsServiceRequest>>>,
    command_output: &impl std::fmt::Display,
) {
    let wait_result = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if !trace_requests.lock().await.is_empty()
                && !metric_requests.lock().await.is_empty()
                && !log_requests.lock().await.is_empty()
            {
                break;
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    if wait_result.is_err() {
        let trace_count = trace_requests.lock().await.len();
        let metric_count = metric_requests.lock().await.len();
        let log_count = log_requests.lock().await.len();

        panic!(
            "timed out waiting for OTLP export from moon (traces={trace_count}, metrics={metric_count}, logs={log_count})\n{command_output}"
        );
    }
}

#[test]
fn shows_otel_flags_in_help() {
    let sandbox = create_empty_moon_sandbox();

    let assert = sandbox.run_bin(|cmd| {
        cmd.arg("--help");
    });

    assert.success().stdout(
        predicate::str::contains("--otel")
            .and(predicate::str::contains("--otel-logs"))
            .and(predicate::str::contains("--otel-service-name")),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exports_task_spans_over_otlp() {
    let sandbox = create_empty_moon_sandbox();
    sandbox.with_default_projects();
    sandbox.enable_git();
    sandbox.create_file(
        "app/moon.yml",
        r#"tasks:
  otel:
    command: bash
    args:
      - -c
      - echo "otel tracing"
"#,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let log_requests = Arc::new(Mutex::new(Vec::new()));
    let metric_requests = Arc::new(Mutex::new(Vec::new()));
    let trace_requests = Arc::new(Mutex::new(Vec::new()));
    let collector = Collector {
        log_requests: Arc::clone(&log_requests),
        metric_requests: Arc::clone(&metric_requests),
        trace_requests: Arc::clone(&trace_requests),
    };
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(
                LogsServiceServer::new(collector.clone())
                    .max_decoding_message_size(32 * 1024 * 1024),
            )
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

    let assert = sandbox.run_bin(|cmd| {
        cmd.arg("--otel")
            .arg("--otel-logs")
            .arg("--otel-service-name")
            .arg("moon-cli-flag-test")
            .arg("run")
            .arg("app:otel");
        cmd.env("MOON_LOG", "debug");
        cmd.env("OTEL_EXPORTER_OTLP_ENDPOINT", format!("http://{addr}"));
        cmd.env(
            "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
            format!("http://{addr}"),
        );
        cmd.env("OTEL_EXPORTER_OTLP_PROTOCOL", "grpc");
        cmd.env("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL", "grpc");
        cmd.env(
            "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
            format!("http://{addr}"),
        );
        cmd.env("OTEL_EXPORTER_OTLP_METRICS_PROTOCOL", "grpc");
        cmd.env("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT", format!("http://{addr}"));
        cmd.env("OTEL_EXPORTER_OTLP_LOGS_PROTOCOL", "grpc");
    });
    let command_output = assert.output();

    assert.success();

    wait_for_cli_exports(
        &trace_requests,
        &metric_requests,
        &log_requests,
        &command_output,
    )
    .await;

    let log_requests = log_requests.lock().await.clone();
    let trace_requests = trace_requests.lock().await.clone();
    let metric_requests = metric_requests.lock().await.clone();
    let spans = trace_requests
        .iter()
        .flat_map(|request| request.resource_spans.iter())
        .flat_map(|resource| resource.scope_spans.iter())
        .flat_map(|scope| scope.spans.iter())
        .collect::<Vec<_>>();
    let trace_service_names = trace_requests
        .iter()
        .flat_map(|request| request.resource_spans.iter())
        .flat_map(|resource| resource.resource.as_ref())
        .flat_map(|resource| resource.attributes.iter())
        .filter(|attribute| attribute.key == "service.name")
        .filter_map(|attribute| attribute.value.as_ref())
        .filter_map(|value| value.value.as_ref())
        .filter_map(|value| match value {
            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value) => {
                Some(value.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let metrics = metric_requests
        .iter()
        .flat_map(|request| request.resource_metrics.iter())
        .flat_map(|resource| resource.scope_metrics.iter())
        .flat_map(|scope| scope.metrics.iter())
        .collect::<Vec<_>>();
    let metric_service_names = metric_requests
        .iter()
        .flat_map(|request| request.resource_metrics.iter())
        .flat_map(|resource| resource.resource.as_ref())
        .flat_map(|resource| resource.attributes.iter())
        .filter(|attribute| attribute.key == "service.name")
        .filter_map(|attribute| attribute.value.as_ref())
        .filter_map(|value| value.value.as_ref())
        .filter_map(|value| match value {
            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value) => {
                Some(value.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let log_bodies = log_requests
        .iter()
        .flat_map(|request| request.resource_logs.iter())
        .flat_map(|resource| resource.scope_logs.iter())
        .flat_map(|scope| scope.log_records.iter())
        .filter_map(|record| record.body.as_ref())
        .filter_map(|body| body.value.as_ref())
        .filter_map(|value| match value {
            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value) => {
                Some(value.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let log_service_names = log_requests
        .iter()
        .flat_map(|request| request.resource_logs.iter())
        .flat_map(|resource| resource.resource.as_ref())
        .flat_map(|resource| resource.attributes.iter())
        .filter(|attribute| attribute.key == "service.name")
        .filter_map(|attribute| attribute.value.as_ref())
        .filter_map(|value| value.value.as_ref())
        .filter_map(|value| match value {
            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value) => {
                Some(value.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    let has_span = |name: &str| spans.iter().any(|span| span.name == name);
    let has_metric = |name: &str| metrics.iter().any(|metric| metric.name == name);
    let span_count = |name: &str| spans.iter().filter(|span| span.name == name).count();
    let observed_span_names = spans
        .iter()
        .map(|span| span.name.as_str())
        .collect::<Vec<_>>();
    let observed_metric_names = metrics
        .iter()
        .map(|metric| metric.name.as_str())
        .collect::<Vec<_>>();
    let task_run = spans.iter().find(|span| span.name == "task_run").unwrap();
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
        "install_dependencies",
        "process_spawn",
        "setup_environment",
        "task_cache_lookup",
        "task_execution",
        "task_hash_generation",
        "task_output_hydration",
        "task_run",
    ];

    assert!(
        has_span("task_run"),
        "observed spans: {observed_span_names:?}"
    );
    assert!(
        has_span("task_concurrency_wait"),
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
        has_metric("moon.task.run.total"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.run.duration"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.concurrency_wait.duration"),
        "observed metrics: {observed_metric_names:?}"
    );
    assert!(
        has_metric("moon.task.cache.lookup.total"),
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
        log_bodies
            .iter()
            .any(|body| body.contains("Running moon") || body.contains("Run target")),
        "expected OTLP logs exported from moon CLI tracing events"
    );
    assert!(!has_span("task_queue_wait"));
    assert_eq!(span_count("task_concurrency_wait"), 1);
    assert_eq!(string_attr(task_run, "task_target"), Some("app:otel"));
    assert_eq!(string_attr(task_run, "cache_source"), Some("miss"));
    assert!(
        trace_service_names.contains(&"moon-cli-flag-test"),
        "expected CLI --otel-service-name to set trace service.name; observed: {trace_service_names:?}"
    );
    assert!(
        metric_service_names.contains(&"moon-cli-flag-test"),
        "expected CLI --otel-service-name to set metric service.name; observed: {metric_service_names:?}"
    );
    assert!(
        log_service_names.contains(&"moon-cli-flag-test"),
        "expected CLI --otel-service-name to set log service.name; observed: {log_service_names:?}"
    );
    assert!(
        metrics.iter().any(|metric| {
            metric.name == "moon.task.run.total"
                && metric_has_string_attr(metric, "task_target", "app:otel")
        }),
        "expected moon.task.run.total to include task_target=app:otel; observed metrics: {observed_metric_names:?}"
    );
    assert!(
        metrics.iter().any(|metric| {
            metric.name == "moon.task.cache.lookup.total"
                && metric_has_string_and_bool_attrs(
                    metric,
                    "cache_source",
                    "miss",
                    "cache_hit",
                    false,
                )
        }),
        "expected moon.task.cache.lookup.total to include cache_source=miss and cache_hit=false; observed metrics: {observed_metric_names:?}"
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
        "exported spans unexpectedly included high-cardinality attrs: {offending_attrs:?}"
    );

    let _ = shutdown_tx.send(());
    server.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exports_task_spans_with_otel_env_vars() {
    let sandbox = create_empty_moon_sandbox();
    sandbox.with_default_projects();
    sandbox.enable_git();
    sandbox.create_file(
        "app/moon.yml",
        r#"tasks:
  otel:
    command: bash
    args:
      - -c
      - echo "otel env tracing"
"#,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let log_requests = Arc::new(Mutex::new(Vec::new()));
    let metric_requests = Arc::new(Mutex::new(Vec::new()));
    let trace_requests = Arc::new(Mutex::new(Vec::new()));
    let collector = Collector {
        log_requests: Arc::clone(&log_requests),
        metric_requests: Arc::clone(&metric_requests),
        trace_requests: Arc::clone(&trace_requests),
    };
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(
                LogsServiceServer::new(collector.clone())
                    .max_decoding_message_size(32 * 1024 * 1024),
            )
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

    let assert = sandbox.run_bin(|cmd| {
        cmd.arg("run").arg("app:otel");
        cmd.env("MOON_LOG", "debug");
        cmd.env("MOON_OTEL", "true");
        cmd.env("MOON_OTEL_LOGS", "true");
        cmd.env("MOON_OTEL_SERVICE_NAME", "moon-cli-env-test");
        cmd.env("OTEL_EXPORTER_OTLP_ENDPOINT", format!("http://{addr}"));
        cmd.env(
            "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
            format!("http://{addr}"),
        );
        cmd.env("OTEL_EXPORTER_OTLP_PROTOCOL", "grpc");
        cmd.env("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL", "grpc");
        cmd.env(
            "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
            format!("http://{addr}"),
        );
        cmd.env("OTEL_EXPORTER_OTLP_METRICS_PROTOCOL", "grpc");
        cmd.env("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT", format!("http://{addr}"));
        cmd.env("OTEL_EXPORTER_OTLP_LOGS_PROTOCOL", "grpc");
    });
    let command_output = assert.output();

    assert.success();

    wait_for_cli_exports(
        &trace_requests,
        &metric_requests,
        &log_requests,
        &command_output,
    )
    .await;

    let log_requests = log_requests.lock().await.clone();
    let trace_requests = trace_requests.lock().await.clone();
    let metric_requests = metric_requests.lock().await.clone();
    let trace_service_names = trace_requests
        .iter()
        .flat_map(|request| request.resource_spans.iter())
        .flat_map(|resource| resource.resource.as_ref())
        .flat_map(|resource| resource.attributes.iter())
        .filter(|attribute| attribute.key == "service.name")
        .filter_map(|attribute| attribute.value.as_ref())
        .filter_map(|value| value.value.as_ref())
        .filter_map(|value| match value {
            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value) => {
                Some(value.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let metric_service_names = metric_requests
        .iter()
        .flat_map(|request| request.resource_metrics.iter())
        .flat_map(|resource| resource.resource.as_ref())
        .flat_map(|resource| resource.attributes.iter())
        .filter(|attribute| attribute.key == "service.name")
        .filter_map(|attribute| attribute.value.as_ref())
        .filter_map(|value| value.value.as_ref())
        .filter_map(|value| match value {
            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value) => {
                Some(value.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let log_service_names = log_requests
        .iter()
        .flat_map(|request| request.resource_logs.iter())
        .flat_map(|resource| resource.resource.as_ref())
        .flat_map(|resource| resource.attributes.iter())
        .filter(|attribute| attribute.key == "service.name")
        .filter_map(|attribute| attribute.value.as_ref())
        .filter_map(|value| value.value.as_ref())
        .filter_map(|value| match value {
            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value) => {
                Some(value.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let log_bodies = log_requests
        .iter()
        .flat_map(|request| request.resource_logs.iter())
        .flat_map(|resource| resource.scope_logs.iter())
        .flat_map(|scope| scope.log_records.iter())
        .filter_map(|record| record.body.as_ref())
        .filter_map(|body| body.value.as_ref())
        .filter_map(|value| match value {
            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value) => {
                Some(value.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        trace_service_names.contains(&"moon-cli-env-test"),
        "expected MOON_OTEL_SERVICE_NAME to set trace service.name; observed: {trace_service_names:?}"
    );
    assert!(
        metric_service_names.contains(&"moon-cli-env-test"),
        "expected MOON_OTEL_SERVICE_NAME to set metric service.name; observed: {metric_service_names:?}"
    );
    assert!(
        log_service_names.contains(&"moon-cli-env-test"),
        "expected MOON_OTEL_SERVICE_NAME to set log service.name; observed: {log_service_names:?}"
    );
    assert!(
        log_bodies
            .iter()
            .any(|body| body.contains("Running moon") || body.contains("Run target")),
        "expected MOON_OTEL_LOGS to enable OTLP log export"
    );

    let _ = shutdown_tx.send(());
    server.await.unwrap();
}
