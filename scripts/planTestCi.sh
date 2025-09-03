#!/usr/bin/env bash

coverage="$COVERAGE"

echo "coverage=$coverage"
echo "coverage=$coverage" >> "$GITHUB_OUTPUT"

# GH Windows is twice as slow as Linux/macOS
osCoverage='["depot-ubuntu-24.04-8","depot-macos-14","depot-windows-2022-16"]'
os='["depot-ubuntu-24.04-4","depot-macos-14","depot-windows-2022-8"]'

if [[ "$coverage" == "true" ]]; then
	echo "os=${osCoverage}"
	echo "os=${osCoverage}" >> "$GITHUB_OUTPUT"
else
	echo "os=${os}"
	echo "os=${os}" >> "$GITHUB_OUTPUT"
fi
