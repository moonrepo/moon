#!/usr/bin/env bash

args="--addFiles --addExports --declaration --declarationConfig tsconfig.build.json"

export NODE_ENV=production

# Build types first since everything depends on it
yarn packemon build --filter @moonrepo/types $args

# Then just build everything
yarn packemon build $args
