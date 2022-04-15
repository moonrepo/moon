#!/usr/bin/env bash

# NOTE: This bumps the version in core and cli packages locally.

bump=${1:-"patch"}

for package in packages/cli packages/core-*; do
	cd "./$package" || exit
	yarn version $bump --deferred
	cd ../..
done
