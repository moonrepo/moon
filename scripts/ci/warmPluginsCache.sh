#!/usr/bin/env bash

# Force warpgate to download + write the cache file sequentially
for plugin in javascript node npm typescript; do
  cargo run --quiet -- toolchain info "$plugin"
done
