#!/usr/bin/env bash
set -e
[ -n "$PROTO_DEBUG" ] && set -x

exec proto run {name} -- "$@"
