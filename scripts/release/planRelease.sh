#!/usr/bin/env bash

channel=latest

if git log -1 --pretty=%B | grep -e "-alpha" -e "-beta" -e "-rc"; then
	channel=next
fi

if [[ "$CANARY" == "true" ]]; then
	channel=canary
fi

if [[ "$NIGHTLY" == "true" ]]; then
	channel=nightly
fi

echo "Setting npm channel to $channel"
echo "npm-channel=$channel" >> "$GITHUB_OUTPUT"
