#!/usr/bin/env bash
set -e
[ -n "$PROTO_DEBUG" ] && set -x

export PROTO_ROOT="{root}"

{{ if install_dir }}
export PROTO_{constant_name}_DIR="{install_dir}"
{{ endif }}

{{ if version }}
export PROTO_{constant_name}_VERSION="{version}"
{{ endif }}

{{ if parent_bin }}
parent="$\{PROTO_{constant_name}_BIN:-{parent_bin}\}"

exec "$parent" "{bin_path}" "$@"
{{ else }}
exec "{bin_path}" "$@"
{{ endif }}
