#!/usr/bin/env bash

# Change the working directory so that we avoid the CLI postinstall checks!
cd scripts

# TODO: this is temp, remove after 0.20 is released
MOON_NODE_VERSION="16.18.0"

# Build all packages with moon itself, so that the order is resolved correctly
npm install -g pnpm
pnpm --package @moonrepo/cli@latest dlx moon run report:build runtime:build types:build

# Note: yarn/npm/npx did not work here, but pnpm does!
