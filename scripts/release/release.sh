#!/usr/bin/env bash
set -euo pipefail

dir=$(dirname "$0")

# Setup npm for publishing
source "$dir/setupNpm.sh"

# Function to map target triple to core package name
getCorePackageFromTriple() {
  local triple="$1"

  case "$triple" in
    aarch64-apple-darwin)
      echo "core-macos-arm64"
      ;;
    aarch64-unknown-linux-gnu)
      echo "core-linux-arm64-gnu"
      ;;
    aarch64-unknown-linux-musl)
      echo "core-linux-arm64-musl"
      ;;
    x86_64-unknown-linux-gnu)
      echo "core-linux-x64-gnu"
      ;;
    x86_64-unknown-linux-musl)
      echo "core-linux-x64-musl"
      ;;
    x86_64-pc-windows-msvc)
      echo "core-windows-x64-msvc"
      ;;
    *)
      echo "Unknown target triple: $triple" >&2
      exit 1
      ;;
  esac
}

# Check for PLAN environment variable
# Shape: cargo dist manifest --output-format=json -a all
if [[ -z "${PLAN}" ]]; then
  echo "Missing dist-manifest PLAN environment variable" >&2
  exit 1
fi

# Parse the PLAN JSON and copy each artifact
echo "Copying artifacts into packages"

echo "$PLAN" | jq -c '.artifacts[]' | while IFS= read -r artifact; do
  kind=$(echo "$artifact" | jq -r '.kind')

  # Skip non-executable-zip artifacts
  if [[ "$kind" != "executable-zip" ]]; then
    continue
  fi

  # Extract artifact details
  name=$(echo "$artifact" | jq -r '.name')
  triple=$(echo "$artifact" | jq -r '.target_triples[0]')

  inputFile="artifacts/$name"
  outputDir="artifacts/release/$triple"

  # Create output directory if it doesn't exist
  mkdir -p "$outputDir"

  # Extract the archive
  if [[ "$inputFile" == *.zip ]]; then
    unzip -q "$inputFile" -d "$outputDir"
  else
    tar -xf "$inputFile" --strip-components=1 -C "$outputDir"
  fi

  # Copy executables to their core package directories
  corePackage=$(getCorePackageFromTriple "$triple")
  exes=("moon" "moonx")

  for exe in "${exes[@]}"; do
    exeName="$exe"

    # Add .exe extension for Windows
    if [[ "$triple" == *windows* ]]; then
      exeName="${exe}.exe"
    fi

    exePath="$outputDir/$exeName"

    if [[ -f "$exePath" ]]; then
      exeDistPath="packages/$corePackage/$exeName"

      cp "$exePath" "$exeDistPath"
      chmod +x "$exeDistPath"

      echo "  Copied $exeDistPath"
    else
      echo "Missing expected executable at path: $exePath" >&2
      exit 1
    fi
  done
done

# We only want to publish packages relating to the Rust binary
tag="${NPM_CHANNEL:-latest}"
version=$(echo "$PLAN" | jq -r '.releases[0].app_version')

if [[ "$version" == *alpha* || "$version" == *beta* || "$version" == *rc* ]]; then
  tag="next"
fi

echo "Publishing cli, core, and types packages"
echo "Tag: $tag"
echo "Version: $version"

if [[ -z "$GITHUB_TOKEN" ]]; then
  echo "Skipping publish step (no GITHUB_TOKEN)"
  exit 0
fi

# We must publish with npm instead of yarn for OIDC to work correctly
for package in packages/cli packages/core-* packages/types; do
	echo "  $package"

  cd "./$package" || exit
  npm publish --tag "$tag" --access public
  cd ../..
done
