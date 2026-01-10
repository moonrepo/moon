#!/usr/bin/env bash
set -eo pipefail

echo "Args: $@"

for ((i=1; i<=$#; i++)); do
  echo "Arg $i: ${!i} (\"${!i}\")"
done
