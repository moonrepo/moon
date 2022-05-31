#!/usr/bin/env bash

dir=$(dirname $0)

source "$dir/../helpers.sh"

target="$TARGET"
package=$(getPackageFromTarget "$target")
binary=$(getBinaryName)

echo "Target: $target"
echo "Package: $package"
echo "Binary: $binary"

# Ensure its "linked" in the package
"$PWD/packages/$package/$binary" --help

