#!/usr/bin/env bash
set -eo pipefail

for var in "${!MOON_@}"; do
	if [[ "$var" != *"MOON_TEST"* && "$var" != *"MOON_VERSION"* ]];then
		echo "$var=${!var}"
	fi
done
