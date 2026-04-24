#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MOON_BIN_OVERRIDE="${MOON_BIN:-}"
MOON_BIN="${MOON_BIN_OVERRIDE:-$ROOT_DIR/target/debug/moon}"
OTELCOL_IMAGE="${OTELCOL_IMAGE:-otel/opentelemetry-collector-contrib:latest}"
CONTAINER_NAME="${CONTAINER_NAME:-moon-otelcol-smoke}"
HOST_OTLP_GRPC_PORT="${HOST_OTLP_GRPC_PORT:-14317}"

TMP_DIR="$(mktemp -d)"
WORKSPACE_DIR="$TMP_DIR/workspace"
CONFIG_PATH="$TMP_DIR/otelcol.yaml"
MOON_STDOUT="$TMP_DIR/moon.stdout"
MOON_STDERR="$TMP_DIR/moon.stderr"

cleanup() {
	docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
	rm -rf "$TMP_DIR"
}

trap cleanup EXIT

print_debug_context() {
	printf '\n--- moon stdout ---\n' >&2
	if [[ -f "$MOON_STDOUT" ]]; then
		sed -n '1,200p' "$MOON_STDOUT" >&2
	fi

	printf '\n--- moon stderr ---\n' >&2
	if [[ -f "$MOON_STDERR" ]]; then
		sed -n '1,200p' "$MOON_STDERR" >&2
	fi

	printf '\n--- collector logs ---\n' >&2
	docker logs "$CONTAINER_NAME" 2>&1 | tail -n 200 >&2 || true

	for file in traces metrics logs; do
		printf '\n--- %s export file ---\n' "$file" >&2
		if [[ -f "$TMP_DIR/$file.json" ]]; then
			tail -n 120 "$TMP_DIR/$file.json" >&2
		fi
	done
}

fail() {
	printf 'collector smoke failed: %s\n' "$*" >&2
	print_debug_context
	exit 1
}

wait_for_container_log() {
	local pattern="$1"
	local description="$2"

	for _ in $(seq 1 30); do
		if docker logs "$CONTAINER_NAME" 2>&1 | grep -Fq "$pattern"; then
			return
		fi

		sleep 1
	done

	fail "timed out waiting for $description ($pattern)"
}

wait_for_file_content() {
	local path="$1"
	local pattern="$2"
	local description="$3"

	for _ in $(seq 1 30); do
		if [[ -f "$path" ]] && grep -Fq "$pattern" "$path"; then
			return
		fi

		sleep 1
	done

	fail "timed out waiting for $description ($pattern) in $path"
}

mkdir -p "$WORKSPACE_DIR/app"
mkdir -p "$WORKSPACE_DIR/.moon"
chmod 0777 "$TMP_DIR"

cat >"$CONFIG_PATH" <<EOF
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
        max_recv_msg_size_mib: 32
processors:
  batch: {}
exporters:
  debug:
    verbosity: detailed
  file/traces:
    path: /data/traces.json
  file/metrics:
    path: /data/metrics.json
  file/logs:
    path: /data/logs.json
service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [debug, file/traces]
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [debug, file/metrics]
    logs:
      receivers: [otlp]
      processors: [batch]
      exporters: [debug, file/logs]
EOF

cat >"$WORKSPACE_DIR/.moon/workspace.yml" <<EOF
projects:
  app: app
EOF

cat >"$WORKSPACE_DIR/app/moon.yml" <<EOF
tasks:
  otel:
    command: bash
    args:
      - -c
      - echo "collector smoke"
EOF

git -C "$WORKSPACE_DIR" init -q
git -C "$WORKSPACE_DIR" config user.email "smoke@example.com"
git -C "$WORKSPACE_DIR" config user.name "moon smoke"
git -C "$WORKSPACE_DIR" add .
git -C "$WORKSPACE_DIR" commit -qm "init smoke workspace"

if [[ -z "$MOON_BIN_OVERRIDE" || ! -x "$MOON_BIN" ]]; then
	cargo build -p moon_cli --bin moon --manifest-path "$ROOT_DIR/Cargo.toml"
fi

docker run -d \
	--name "$CONTAINER_NAME" \
	-p "$HOST_OTLP_GRPC_PORT:4317" \
	-v "$CONFIG_PATH:/etc/otelcol/config.yaml:ro" \
	-v "$TMP_DIR:/data" \
	"$OTELCOL_IMAGE" \
	--config=/etc/otelcol/config.yaml >/dev/null

wait_for_container_log "Everything is ready" "collector readiness"

if ! (
	cd "$WORKSPACE_DIR"
	MOON_LOG=debug \
	OTEL_EXPORTER_OTLP_PROTOCOL=grpc \
	OTEL_EXPORTER_OTLP_TRACES_PROTOCOL=grpc \
	OTEL_EXPORTER_OTLP_METRICS_PROTOCOL=grpc \
	OTEL_EXPORTER_OTLP_LOGS_PROTOCOL=grpc \
	OTEL_EXPORTER_OTLP_ENDPOINT="http://127.0.0.1:$HOST_OTLP_GRPC_PORT" \
	OTEL_EXPORTER_OTLP_TRACES_ENDPOINT="http://127.0.0.1:$HOST_OTLP_GRPC_PORT" \
	OTEL_EXPORTER_OTLP_METRICS_ENDPOINT="http://127.0.0.1:$HOST_OTLP_GRPC_PORT" \
	OTEL_EXPORTER_OTLP_LOGS_ENDPOINT="http://127.0.0.1:$HOST_OTLP_GRPC_PORT" \
	"$MOON_BIN" --otel --otel-logs --otel-service-name moon-collector-smoke run app:otel >"$MOON_STDOUT" 2>"$MOON_STDERR"
); then
	fail "moon command failed"
fi

if ! docker stop "$CONTAINER_NAME" >/dev/null; then
	fail "collector did not stop cleanly"
fi

wait_for_file_content "$TMP_DIR/traces.json" "task_run" "task span export"
wait_for_file_content "$TMP_DIR/metrics.json" "moon.task.run.total" "task metric export"
wait_for_file_content "$TMP_DIR/logs.json" "Running moon" "log export"

printf 'collector smoke passed\n'
