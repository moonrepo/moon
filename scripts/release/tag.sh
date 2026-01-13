#!/usr/bin/env bash
set -euo pipefail

version="$1"

if [[ -z "$version" ]]; then
  echo "Missing version to release!" >&2
  exit 1
fi

echo "Version: $version"

echo "Bumping npm packages"

# Bump npm versions first so that cargo will commit the changes
yarn workspaces foreach -tvR --from "@moonrepo/{cli,core-*}" version "$version"

echo "Bumping cargo crate"

cargo release "$version" --no-publish -p moon_cli

