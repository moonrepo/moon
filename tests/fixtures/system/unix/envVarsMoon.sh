#!/usr/bin/env bash
set -eo pipefail

for var in "${!MOON_@}"; do
	if [[ "$var" != *"MOON_TEST"* && "$var" != *"MOON_VERSION"* && "$var" != *"MOON_APP_LOG"* ]];then
		echo "$var=${!var}"
	fi
done
