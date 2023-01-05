#!/usr/bin/env bash
set -e
[ -n "$PROTO_DEBUG" ] && set -x

export PROTO_ROOT="{root}"

{{ if install_dir }}
export PROTO_{name | uppercase}_DIR="{install_dir}"
{{ endif }}

{{ if version }}
export PROTO_{name | uppercase}_VERSION="{version}"
{{ endif }}

{{ if parent_name }}
parent="$\{PROTO_{parent_name | uppercase}_BIN:-{parent_name}}"

exec "$parent" "{bin_path}" "$@"

{{ else }}
exec "{bin_path}" "$@"
{{ endif }}
