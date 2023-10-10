#!/usr/bin/env bash

# Determine release channel. If contains "alpha", "beta", or "rc",
# then publish to next, otherwise latest.
channel=unknown

if git log -1 --pretty=%B | grep -e "-alpha" -e "-beta" -e "-rc"; then
	channel=next
fi

if [[ "$NIGHTLY" == "true" ]]; then
	channel=nightly
fi

echo "Setting npm channel to $channel"
echo "npm-channel=$channel" >> "$GITHUB_OUTPUT"
