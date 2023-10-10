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

if [[ "${NIGHTLY}" == "true" ]]; then
	echo "Nightly build detected, appending timestamp to versions"
	timestamp=$(date +%Y%m%d)

	for package in packages/*; do
		echo "$package"
		cd "./$package" || exit
		pkg=$(jq ".version += \"-nightly.$timestamp\"" package.json)
		echo "$pkg"
		echo "$pkg" > package.json
		cd ../..
	done
fi
