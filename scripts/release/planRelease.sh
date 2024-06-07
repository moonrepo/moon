#!/usr/bin/env bash

# Determine channel to publish to

channel=latest

if git log -1 --pretty=%B | grep -e "-alpha" -e "-beta" -e "-rc"; then
	channel=next
fi

if [[ "$CANARY" == "true" ]]; then
	channel=canary
fi

if [[ "$NIGHTLY" == "true" ]]; then
	channel=nightly
fi

echo "Setting npm channel to $channel"
echo "npm-channel=$channel" >> "$GITHUB_OUTPUT"

# Extract the CLI version being published

baseVersion=$(jq -r ".version" packages/cli/package.json)
version="$baseVersion"
build=""

if [[ "$channel" == "canary" || "$channel" == "nightly" ]]; then
	build="-$channel.$(date +%Y%m%d%H%M)"
	version="$version$build"
fi

echo "Setting cli version to $version (base: $baseVersion, build: $build)"
echo "cli-version=$version" >> "$GITHUB_OUTPUT"
echo "cli-version-base=$baseVersion" >> "$GITHUB_OUTPUT"
echo "cli-version-build=$build" >> "$GITHUB_OUTPUT"
