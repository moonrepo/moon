#!/usr/bin/env bash

coverage="$COVERAGE"

echo "coverage=$coverage"
echo "coverage=$coverage" >> "$GITHUB_OUTPUT"

# GH Windows is twice as slow as Linux/macOS
osCoverage='["depot-ubuntu-22.04-4","macos-latest","depot-windows-2022-4"]'
os='["ubuntu-latest","macos-latest","depot-windows-2022-4"]'

if [[ "$coverage" == "true" ]]; then
	echo "os=${osCoverage}"
	echo "os=${osCoverage}" >> "$GITHUB_OUTPUT"
else
	echo "os=${os}"
	echo "os=${os}" >> "$GITHUB_OUTPUT"
fi
