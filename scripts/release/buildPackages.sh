#!/usr/bin/env bash

# Build all packages with moon itself, so that the order is resolved correctly
yarn dlx --package @moonrepo/cli@latest moon run :build

# # When not in CI, use npx
# if [[ -z "${CI}" ]]; then
# 	npx --package @moonrepo/cli@latest -- moon run :build

# # npx doesn't work in CI, so install directly
# else
# 	yarn add --dev @moonrepo/cli@latest
# 	./node_modules/@moonrepo/cli/moon run :build
# fi
