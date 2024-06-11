#!/usr/bin/env bash

dir=$(dirname $0)

export RUSTFLAGS="-C strip=symbols"

source "$dir/../helpers.sh"

target="$TARGET"
oldVersion="$CLI_VERSION_BASE"
newVersion="$CLI_VERSION"

# Set the cli version before building (it may change for canary/nightly)

echo "Old version: $oldVersion"
echo "New version: $newVersion"

if [[ "$oldVersion" != "$newVersion" ]]; then
	toml=$(cat crates/cli/Cargo.toml)
	echo "${toml//$oldVersion/$newVersion}" > crates/cli/Cargo.toml
fi

# Build the binary with the provided target

echo "Target: $target"
echo "Args: $@"

rustup target add "$target"
cargo build --release --target "$target" $@
