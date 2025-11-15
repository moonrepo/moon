#!/usr/bin/env bash
set -eo pipefail

for var in "${!MOON_@}"; do
	echo "$var=${!var}"
done
