#!/usr/bin/env bash

# Ensure the CLI binary has not been accidently modified and pushed
size=$(stat -c %s packages/cli/moon)
# size=$(stat -f "%z" packages/cli/moon) # macos

if [[ "$size" -gt "70" ]]; then
	echo "Binary 'packages/cli/moon' was modified, please revert!"
	exit 1
fi
