#!/usr/bin/env bash
set -euo pipefail

version="$1"

if [[ -z "$version" ]]; then
  echo "Missing version to release!" >&2
  exit 1
fi

echo "Version: $version"

echo "Bumping npm packages"

yarn workspaces foreach -tvR --from "@moonrepo/{cli,core-*}" version "$version"

# Cargo release will fail with a "dirty working tree" if we have uncommitted changes,
# so unfortunately we need another commit here...
git add --all
git commit -m "chore: Bump packages"

echo "Bumping cargo crate"

cargo release "$version" --no-publish -p moon_cli

