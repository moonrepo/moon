#!/usr/bin/env bash
set -eo pipefail

for var in "${!MOON_@}"; do
	if [[ "$var" != *"MOON_TEST"* ]];then
		echo "$var=${!var}"
	fi
done
