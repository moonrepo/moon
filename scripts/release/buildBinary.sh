#!/usr/bin/env bash

dir=$(dirname $0)

export RUSTFLAGS="-C strip=symbols"

source "$dir/../helpers.sh"

target="$TARGET"

echo "Target: $target"
echo "Args: $@"

# Build the binary with the provided target
rustup target add "$target"
cargo build --release --target "$target" $@
