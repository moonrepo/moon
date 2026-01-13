#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${NPM_TOKEN}" ]]; then
	echo "Missing NPM_TOKEN!" >&2
else
	echo "//registry.npmjs.org/:_authToken=$NPM_TOKEN" >> ~/.npmrc
	echo "npmAuthToken: $NPM_TOKEN" >> ./.yarnrc.yml
fi

if [[ -d ".yarn/versions" ]]; then
	echo "Yarn versions detected, applying updates"
	yarn version apply --all
fi

