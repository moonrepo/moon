#!/usr/bin/env bash

bump=${1:-"patch"}

for package in packages/*; do
	echo "$package -> $bump"
	cd "./$package" || exit
	yarn version "$bump" --deferred
	cd ../..
done
