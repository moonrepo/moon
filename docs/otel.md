# OpenTelemetry

`moon` can now export 3 OTLP signals:

- traces: `MOON_OTEL=true`
- metrics: `MOON_OTEL=true`
- logs: `MOON_OTEL_LOGS=true`

All exporters use the standard `OTEL_EXPORTER_OTLP_*` environment variables. The common service
name override is `MOON_OTEL_SERVICE_NAME`.

Example:

```bash
MOON_LOG=debug \
MOON_OTEL=true \
MOON_OTEL_LOGS=true \
MOON_OTEL_SERVICE_NAME=moon-dev \
OTEL_EXPORTER_OTLP_PROTOCOL=grpc \
OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317 \
moon run app:build
```

## Exported data

Traces currently include task lifecycle spans such as:

- `task_run`
- `task_hash_generation`
- `task_cache_lookup`
- `task_output_hydration`
- `task_execution`
- `task_output_archive`
- `task_concurrency_wait`

Metrics currently include:

- `moon.task.run.total`
- `moon.task.run.failure.total`
- `moon.task.run.duration`
- `moon.task.hash_generation.duration`
- `moon.task.cache.lookup.total`
- `moon.task.cache.hit.total`
- `moon.task.cache.lookup.duration`
- `moon.task.output.hydration.duration`
- `moon.task.output.archive.duration`
- `moon.task.execution.duration`
- `moon.task.execution.attempts`
- `moon.task.retry.total`
- `moon.task.concurrency_wait.duration`

Logs are exported by bridging `tracing` events into OTLP log records. This is separate from the
existing stderr/file log output, so local logs still work even when OTLP logs are disabled.

## Real collector smoke tests

These scripts are opt-in and Docker-based. They are meant for local verification, not normal CI.

- OTel Collector smoke:
  - [scripts/otel/smoke-collector.sh](../scripts/otel/smoke-collector.sh)
  - Proves OTLP traces, metrics, and logs are accepted by a real OpenTelemetry Collector and show
    up in the collector debug exporter output.
- Alloy + Loki smoke:
  - [scripts/otel/smoke-alloy-loki.sh](../scripts/otel/smoke-alloy-loki.sh)
  - Proves Alloy accepts OTLP traces, metrics, and logs, and that OTLP logs land in Loki and are
    queryable through the Loki HTTP API.

Both scripts accept image overrides through env vars so versions can be pinned when needed.
