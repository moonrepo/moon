#!/usr/bin/env bash

dir=$(dirname $0)

source "$dir/../helpers.sh"

mkdir -p "$PWD/artifacts"

for file in website/static/schemas/*; do
	echo "$file"

	cp "$file" "$PWD/artifacts/schema-$(basename $file)"
done
