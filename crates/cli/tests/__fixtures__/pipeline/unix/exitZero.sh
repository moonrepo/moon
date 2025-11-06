#!/usr/bin/env bash
set -eo pipefail

echo "stdout"
echo "stderr" >&2

exit 0

echo "This should not appear!"
