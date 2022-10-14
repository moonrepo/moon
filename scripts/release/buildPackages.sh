#!/usr/bin/env bash

# Change the working directory so that we avoid the CLI postinstall checks!
cd scripts

# Build all packages with moon itself, so that the order is resolved correctly
npm install -g pnpm
pnpm --package @moonrepo/cli@latest dlx moon run :build

# Note: yarn/npm/npx did not work here, but pnpm does!
