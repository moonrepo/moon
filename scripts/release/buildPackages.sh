#!/usr/bin/env bash

# Build all packages with moon itself, so that the order is resolved correctly
npx --package @moonrepo/cli@latest -- moon run :build
