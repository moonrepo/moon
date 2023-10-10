#!/usr/bin/env bash

dir=$(dirname $0)
tag="${NPM_CHANNEL:-latest}"

# Setup npm for publishing
source "$dir/setupNpm.sh"

# We only want to publish packages NOT relating to the Rust binary
for package in packages/*; do
	echo "$package"

	if [[ ("$package" == *"cli"*) || ("$package" == *"core"*) ]]; then
		# Ignore cli/core packages
		echo "Skipping"
	elif [[ -z "${GITHUB_TOKEN}" ]]; then
		# Testing locally
		echo "Not publishing"
	else
		cd "./$package" || exit
		# We can't use npm because of: https://github.com/npm/cli/issues/2610
		yarn npm publish --tag "$tag" --access public
		cd ../..
	fi
done
