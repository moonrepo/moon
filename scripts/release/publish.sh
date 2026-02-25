#!/usr/bin/env bash
set -euo pipefail

dir=$(dirname "$0")

# Setup npm for publishing
source "$dir/setupNpm.sh"

# Bump versions if applicable
if [[ -d ".yarn/versions" ]]; then
	echo "Yarn versions detected, applying updates"
	yarn version apply --all
fi

# We only want to publish packages NOT relating to the Rust binary
tag="${NPM_CHANNEL:-latest}"

echo "Publishing secondary packages"
echo "Tag: $tag"

if [[ -z "$GITHUB_TOKEN" ]]; then
  echo "Skipping publish step (no GITHUB_TOKEN)"
  exit 0
fi

# We must publish with npm instead of yarn for OIDC to work correctly
for package in packages/*; do
	echo "  $package"

	# Ignore cli/core/types packages
	if [[ ("$package" == *"cli"*) || ("$package" == *"core"*) || ("$package" == *"types"*) ]]; then
		echo "Skipping"
	else
		cd "./$package" || exit
		npm publish --tag "$tag" --access public
		cd ../..
	fi
done
