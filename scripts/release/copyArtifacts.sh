#!/usr/bin/env bash
set -euo pipefail

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
if [[ -z "${PLAN}" ]]; then
  echo "Missing dist-manifest PLAN environment variable" >&2
  exit 1
fi

# Parse the PLAN JSON and process each artifact
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
  for exe in moon moonx; do
    exeName="$exe"

    # Add .exe extension for Windows
    if [[ "$triple" == *windows* ]]; then
      exeName="${exe}.exe"
    fi

    exePath="$outputDir/$exeName"

    if [[ -f "$exePath" ]]; then
      corePackage=$(getCorePackageFromTriple "$triple")

      cp "$exePath" "packages/$corePackage/$exeName"
    else
      echo "Missing expected executable at path: $exePath" >&2
      exit 1
    fi
  done
done
