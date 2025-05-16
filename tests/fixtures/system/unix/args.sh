#!/usr/bin/env bash
set -eo pipefail

for ((i=1; i<=$#; i++)); do
  echo "Arg $i: ${!i} (\"${!i}\")"
done
