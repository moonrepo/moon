#!/usr/bin/env bash

bump=${1:-"patch"}

for package in packages/*; do
	cd "./$package" || exit
	echo "$package -> $bump"
	yarn version $bump --deferred
	cd ../..
done
