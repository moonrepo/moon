#!/usr/bin/env bash

name=$(cat packages/cli/package.json | jq -r '.name')
version=$(cat packages/cli/package.json | jq -r '.version')
tag="$name@$version"

if [[ -z "${NPM_TOKEN}" ]]; then
	echo "$tag"
else
	# Update env var in GitHub actions
	echo "NPM_TAG_NAME=$tag" >> $GITHUB_ENV
fi
