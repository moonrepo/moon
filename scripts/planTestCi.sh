#!/usr/bin/env bash

coverage="$COVERAGE"

# Windows runs out of disk space
if [[ "$OS" == "Windows" ]]; then
	coverage="false"
fi

echo "coverage=$coverage" >> "$GITHUB_OUTPUT"

osCoverage='["depot-ubuntu-22.04-4","macos-latest","windows-latest"]'
os='["ubuntu-latest","macos-latest","windows-latest"]'

if [[ "$coverage" == "true" ]]; then
	echo "os='${osCoverage}'" >> "$GITHUB_OUTPUT"
else
	echo "os='${os}'" >> "$GITHUB_OUTPUT"
fi
