#!/usr/bin/env bash

dir=$(dirname $0)

# Setup npm for publishing
source "$dir/setupNpm.sh"

# We only want to publish packages NOT relating to the Rust binary
for package in packages/*; do
	if [[ ("$package" == *"cli"*) || ("$package" == *"core"*) ]]; then
		# Ignore cli/core packages
		echo "Skipping $package"
	elif [[ -z "${GITHUB_TOKEN}" ]]; then
		# Testing locally
		echo $package
	else
		cd "./$package" || exit
		# We cant use npm because of: https://github.com/npm/cli/issues/2610
		yarn npm publish --tag $tag --access public
		cd ../..
	fi
done
