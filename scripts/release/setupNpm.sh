#!/usr/bin/env bash

# Determine release channel. If contains "alpha", "beta", or "rc",
# then publish to next, otherwise latest.
channel=latest

if git log -1 --pretty=%B | grep -e "-alpha" -e "-beta" -e "-rc"; then
	channel=next
fi

if [[ -z "${NPM_TOKEN}" ]]; then
	echo "Missing NPM_TOKEN!"
else
	echo "//registry.npmjs.org/:_authToken=$NPM_TOKEN" >> ~/.npmrc
	echo "npmAuthToken: $NPM_TOKEN" >> ./.yarnrc.yml

	# Update env var in GitHub actions
	echo "NPM_CHANNEL=$channel" >> $GITHUB_ENV
fi

# And make it available to other scripts
export NPM_CHANNEL="$channel"
