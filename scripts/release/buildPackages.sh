#!/usr/bin/env bash

# Change the working directory so that we avoid the CLI postinstall checks!
cd scripts

# Build all packages with moon itself, so that the order is resolved correctly
npx --package @moonrepo/cli@latest -- moon run :build

# # When not in CI, use npx
# if [[ -z "${CI}" ]]; then
# 	npx --package @moonrepo/cli@latest -- moon run :build

# # npx doesn't work in CI, so install directly
# else
# 	yarn add --dev @moonrepo/cli@latest
# 	./node_modules/@moonrepo/cli/moon run :build
# fi
