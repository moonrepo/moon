#!/usr/bin/env bash

args="--addFiles --addExports --declaration"

export NODE_ENV=production

# Build types first since everything depends on it
yarn packemon build-workspace --filter @moonrepo/types $args

# Then just build everything
yarn packemon build-workspace $args

# Then build the visualizer with vite
yarn workspace @moonrepo/visualizer run build
