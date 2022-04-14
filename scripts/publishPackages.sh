#!/usr/bin/env bash

# Determine release channel. If contains "alpha", "beta", or "rc",
# then publish to next, otherwise latest.
tag=latest

if git log -1 --pretty=%B | grep -e "-alpha" -e "-beta" -e "-rc"; then
	tag=next
fi

if [[ -z "${NPM_TOKEN}" ]]; then
	echo "Missing NPM_TOKEN!"
else
	echo "//registry.npmjs.org/:_authToken=$NPM_TOKEN" >> ~/.npmrc

	# Update env var in GitHub actions
	echo "NPM_CHANNEL=$tag" >> $GITHUB_ENV
fi

# We only want to publish packages relating to the Rust binary.
# Other packages will be published the classic way.
for package in packages/cli packages/core-*; do
	if [[ -z "${GITHUB_TOKEN}" ]]; then
		echo $package # Testing locally
	else
		cd "./$package" || exit
		# We cant use npm because of: https://github.com/npm/cli/issues/2610
		yarn npm publish --tag $tag --access public
		cd ../..
	fi
done
