#!/usr/bin/env bash

dir=$(dirname $0)

source "$dir/../helpers.sh"

target="$TARGET"
package=$(getPackageFromTarget "$target")
binary=$(getBinaryName)

echo "Target: $target"
echo "Package: $package"
echo "Binary: $binary"

targetBin="$PWD/target/$target/release/$binary"
packageBin="$PWD/packages/$package/$binary"
artifactBin="$PWD/$binary"

# Copy the binary to the package
cp "$targetBin" "$packageBin"
chmod +x "$packageBin"

# Copy into root so that it can be uploaded as an artifact
cp "$targetBin" "$artifactBin"
chmod +x "$artifactBin"
