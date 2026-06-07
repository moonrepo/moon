#!/usr/bin/env bash

export NODE_ENV=production

# Build types first since everything depends on it
vp pack packages/types

# Then just build everything
vp pack

# Then build the visualizer with vite
yarn workspace @moonrepo/visualizer run build
yarn workspace website run build
