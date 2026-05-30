#!/usr/bin/env bash
set -euo pipefail

export PROTO_HOME="$HOME/.proto-tests"

# Legacy tests
proto install node 18.0.0
proto install npm 8.19.0
