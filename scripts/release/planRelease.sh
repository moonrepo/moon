#!/usr/bin/env bash

# Determine release channel. If contains "alpha", "beta", or "rc",
# then publish to next, otherwise latest.
channel=unknown

if git log -1 --pretty=%B | grep -e "-alpha" -e "-beta" -e "-rc"; then
	channel=next
fi

echo "Setting npm channel to $channel"

# Update env var in GitHub actions
echo "NPM_CHANNEL=$channel" >> "$GITHUB_ENV"

# And make it available to other scripts
export NPM_CHANNEL="$channel"
