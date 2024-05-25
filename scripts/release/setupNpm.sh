#!/usr/bin/env bash

if [[ -z "${NPM_TOKEN}" ]]; then
	echo "Missing NPM_TOKEN!"
else
	echo "//registry.npmjs.org/:_authToken=$NPM_TOKEN" >> ~/.npmrc
	echo "npmAuthToken: $NPM_TOKEN" >> ./.yarnrc.yml
fi

if [ -d ".yarn/versions" ]; then
	echo "Yarn versions detected, applying updates"
	yarn version apply --all
fi

if [[ "$NPM_CHANNEL" == "canary" || "$NPM_CHANNEL" == "nightly" ]]; then
	timestamp=$(date +%Y%m%d%H%M)
	preid="-$NPM_CHANNEL.$timestamp"

	echo "Detected \"$NPM_CHANNEL\" build, appending timestamp to versions"
	echo "Prerelease: $preid"

	for package in packages/*; do
		echo "$package"
		cd "./$package" || exit

		# For the cli package, replace itself and all dep versions
		if [[ "$package" == *"cli"* ]]; then
			baseVersion=$(jq -r ".version" package.json)
			version="$baseVersion$preid"

			pkg=$(jq ".version += \"$preid\"" package.json)
			echo "${pkg//$baseVersion/$version}" > package.json


		# For core packages, append the preid to the version
		else
			pkg=$(jq ".version += \"$preid\"" package.json)
			echo "$pkg" > package.json
		fi

		# Print it out so we can debug it
		echo $(cat package.json)

		cd ../..
	done
fi
