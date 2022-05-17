#!/usr/bin/env bash

dir=$(dirname $0)
tag="${NPM_CHANNEL:-latest}"

# Setup npm for publishing
source "$dir/setupNpm.sh"

# We only want to publish packages relating to the Rust binary
for package in packages/cli packages/core-*; do
	echo "$package"

	if [[ -z "${GITHUB_TOKEN}" ]]; then
		# Testing locally
		echo "Not publishing"
	else
		cd "./$package" || exit
		# We cant use npm because of: https://github.com/npm/cli/issues/2610
		yarn npm publish --tag "$tag" --access public
		cd ../..
	fi
done

# Set the tag to use for GitHub releases
name=$(cat packages/cli/package.json | jq -r '.name')
version=$(cat packages/cli/package.json | jq -r '.version')
tag="$name@$version"

echo "NPM_TAG_NAME=$tag" >> $GITHUB_ENV
export NPM_TAG_NAME="$tag"
