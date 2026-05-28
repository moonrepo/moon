#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MOON_BIN_OVERRIDE="${MOON_BIN:-}"
MOON_BIN="${MOON_BIN_OVERRIDE:-$ROOT_DIR/target/debug/moon}"
ALLOY_IMAGE="${ALLOY_IMAGE:-grafana/alloy:latest}"
LOKI_IMAGE="${LOKI_IMAGE:-grafana/loki:latest}"
NETWORK_NAME="${NETWORK_NAME:-moon-otel-smoke}"
ALLOY_CONTAINER_NAME="${ALLOY_CONTAINER_NAME:-moon-alloy-smoke}"
LOKI_CONTAINER_NAME="${LOKI_CONTAINER_NAME:-moon-loki-smoke}"
HOST_OTLP_GRPC_PORT="${HOST_OTLP_GRPC_PORT:-14318}"
HOST_LOKI_PORT="${HOST_LOKI_PORT:-13100}"

TMP_DIR="$(mktemp -d)"
WORKSPACE_DIR="$TMP_DIR/workspace"
ALLOY_CONFIG_PATH="$TMP_DIR/config.alloy"
MOON_STDOUT="$TMP_DIR/moon.stdout"
MOON_STDERR="$TMP_DIR/moon.stderr"
LOKI_RESPONSE_PATH="$TMP_DIR/loki.response.json"

cleanup() {
	docker rm -f "$ALLOY_CONTAINER_NAME" "$LOKI_CONTAINER_NAME" >/dev/null 2>&1 || true
	docker network rm "$NETWORK_NAME" >/dev/null 2>&1 || true
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

	printf '\n--- alloy logs ---\n' >&2
	docker logs "$ALLOY_CONTAINER_NAME" 2>&1 | tail -n 200 >&2 || true

	printf '\n--- loki logs ---\n' >&2
	docker logs "$LOKI_CONTAINER_NAME" 2>&1 | tail -n 200 >&2 || true

	printf '\n--- loki response ---\n' >&2
	if [[ -f "$LOKI_RESPONSE_PATH" ]]; then
		sed -n '1,200p' "$LOKI_RESPONSE_PATH" >&2
	fi
}

fail() {
	printf 'alloy+loki smoke failed: %s\n' "$*" >&2
	print_debug_context
	exit 1
}

wait_for_tcp_port() {
	local host="$1"
	local port="$2"
	local description="$3"

	for _ in $(seq 1 30); do
		if (echo >"/dev/tcp/$host/$port") >/dev/null 2>&1; then
			return
		fi

		sleep 1
	done

	fail "timed out waiting for $description on $host:$port"
}

wait_for_container_log() {
	local container="$1"
	local pattern="$2"
	local description="$3"
	local log_path="$TMP_DIR/$container.log"

	for _ in $(seq 1 45); do
		if docker logs "$container" >"$log_path" 2>&1 && grep -Fq "$pattern" "$log_path"; then
			return
		fi

		sleep 1
	done

	fail "timed out waiting for $description ($pattern)"
}

wait_for_loki_ready() {
	for _ in $(seq 1 45); do
		if curl -fsS "http://127.0.0.1:$HOST_LOKI_PORT/ready" >/dev/null 2>&1; then
			return
		fi

		sleep 1
	done

	fail "timed out waiting for Loki readiness"
}

wait_for_loki_log() {
	for _ in $(seq 1 45); do
		if curl -fsS -G "http://127.0.0.1:$HOST_LOKI_PORT/loki/api/v1/query_range" \
			--data-urlencode 'query={service_name="moon-alloy-loki-smoke"} |= "Running moon"' \
			--data-urlencode 'limit=20' >"$LOKI_RESPONSE_PATH" 2>/dev/null &&
			grep -Fq 'Running moon' "$LOKI_RESPONSE_PATH"; then
			return
		fi

		sleep 1
	done

	fail "timed out waiting for exported moon logs in Loki"
}

mkdir -p "$WORKSPACE_DIR/.moon" "$WORKSPACE_DIR/app"

cat >"$ALLOY_CONFIG_PATH" <<EOF
otelcol.receiver.otlp "default" {
  grpc {
    endpoint = "0.0.0.0:4317"
    max_recv_msg_size = "32MiB"
  }

  output {
    traces  = [otelcol.processor.batch.default.input]
    metrics = [otelcol.processor.batch.default.input]
    logs = [otelcol.processor.batch.default.input]
  }
}

otelcol.processor.batch "default" {
  output {
    traces  = [otelcol.exporter.debug.default.input]
    metrics = [otelcol.exporter.debug.default.input]
    logs    = [otelcol.exporter.debug.default.input, otelcol.exporter.otlphttp.loki.input]
  }
}

otelcol.exporter.debug "default" {
  verbosity = "detailed"
}

otelcol.exporter.otlphttp "loki" {
  client {
    // Loki accepts OTLP logs on /otlp/v1/logs when the exporter endpoint is /otlp.
    endpoint = "http://$LOKI_CONTAINER_NAME:3100/otlp"
  }
}
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
      - echo "alloy loki smoke"
EOF

git -C "$WORKSPACE_DIR" init -q
git -C "$WORKSPACE_DIR" config user.email "smoke@example.com"
git -C "$WORKSPACE_DIR" config user.name "moon smoke"
git -C "$WORKSPACE_DIR" add .
git -C "$WORKSPACE_DIR" commit -qm "init smoke workspace"

if [[ -z "$MOON_BIN_OVERRIDE" || ! -x "$MOON_BIN" ]]; then
	cargo build -p moon_cli --bin moon --manifest-path "$ROOT_DIR/Cargo.toml"
fi

docker network create "$NETWORK_NAME" >/dev/null

docker run -d \
	--name "$LOKI_CONTAINER_NAME" \
	--network "$NETWORK_NAME" \
	-p "$HOST_LOKI_PORT:3100" \
	"$LOKI_IMAGE" \
	-config.file=/etc/loki/local-config.yaml >/dev/null

wait_for_loki_ready

docker run -d \
	--name "$ALLOY_CONTAINER_NAME" \
	--network "$NETWORK_NAME" \
	-p "$HOST_OTLP_GRPC_PORT:4317" \
	-v "$ALLOY_CONFIG_PATH:/etc/alloy/config.alloy:ro" \
	"$ALLOY_IMAGE" \
	run --stability.level=experimental /etc/alloy/config.alloy >/dev/null

wait_for_tcp_port "127.0.0.1" "$HOST_OTLP_GRPC_PORT" "Alloy OTLP gRPC receiver"

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
	"$MOON_BIN" --otel --otel-logs --otel-service-name moon-alloy-loki-smoke run app:otel >"$MOON_STDOUT" 2>"$MOON_STDERR"
); then
	fail "moon command failed"
fi

wait_for_container_log "$ALLOY_CONTAINER_NAME" "Span #" "trace export"
wait_for_container_log "$ALLOY_CONTAINER_NAME" "moon.task.run.total" "task metric export"
wait_for_container_log "$ALLOY_CONTAINER_NAME" "Running moon" "log export"
wait_for_loki_log

printf 'alloy+loki smoke passed\n'
