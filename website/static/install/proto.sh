#!/usr/bin/env bash
# Copyright 2022 moonrepo, Inc.

# Thanks to Deno for the original script:
# https://github.com/denoland/deno_install/blob/master/install.sh

set -eo pipefail

bin="proto"
shim_bin="proto-shim"
arch=$(uname -sm)
version="${1:-latest}"
ext=".tar.xz"

if [[ "$OS" == "Windows_NT" ]]; then
	target="proto_cli-x86_64-pc-windows-msvc"
	bin="proto.exe"
	shim_bin="proto-shim.exe"
	ext=".zip"
else
	case "$arch" in
	"Darwin x86_64") target="proto_cli-x86_64-apple-darwin" ;;
	"Darwin arm64") target="proto_cli-aarch64-apple-darwin" ;;
	"Linux aarch64") target="proto_cli-aarch64-unknown-linux" ;;
	"Linux x86_64") target="proto_cli-x86_64-unknown-linux" ;;
	*)
		echo "Unsupported system or architecture \"$arch\". Unable to install proto!"
		exit 1
	 ;;
	esac
fi

if [[ "$arch" == "Linux"* ]]; then
	deps=$(ldd --version 2>&1 || true)

	if [[ $deps == *"musl"* ]]; then
		target="$target-musl"
	else
		target="$target-gnu"
	fi
fi

wsl=$(uname -a)
if [[ "$wsl" == *"Microsoft"* || "$wsl" == *"microsoft"* ]]; then
  is_wsl=true
else
  is_wsl=false
fi

if [[ "$version" == "latest" ]]; then
	download_url="https://github.com/moonrepo/proto/releases/latest/download/$target$ext"
else
	download_url="https://github.com/moonrepo/proto/releases/download/v$version/$target$ext"
fi

temp_dir="$HOME/.proto/temp/proto/$target"
download_file="$temp_dir$ext"

if [[ -z "$PROTO_INSTALL_DIR" ]]; then
	install_dir="$HOME/.proto/bin"
else
	install_dir="$PROTO_INSTALL_DIR"
fi

bin_path="$install_dir/$bin"
shim_path="$install_dir/$shim_bin"

# Download and unpack in temp dir

if [[ ! -d "$temp_dir" ]]; then
	mkdir -p "$temp_dir"
fi

curl --fail --location --progress-bar --output "$download_file" "$download_url"

if [[ "$ext" == ".zip" ]]; then
	unzip -d "$temp_dir" "$download_file"

	# Unzip doesnt remove components folder
	temp_dir="$temp_dir/$target"
else
	tar xf "$download_file" --strip-components 1 -C "$temp_dir"
fi

# Move to bin dir and clean up

if [[ ! -d "$install_dir" ]]; then
	mkdir -p "$install_dir"
fi

mv "$temp_dir/$bin" "$bin_path"
chmod +x "$bin_path"

if [[ -f "$temp_dir/$shim_bin" ]]; then
	mv "$temp_dir/$shim_bin" "$shim_path"
	chmod +x "$shim_path"
fi

rm -rf "$download_file" "$temp_dir"

# Run setup script to update shells

export PROTO_LOG=error
$bin_path setup

if [[ "$PROTO_DEBUG" == "true" ]]; then
	echo
	echo "arch=$arch"
	echo "target=$target"
	echo "download_url=$download_url"
	echo "bin_path=$bin_path"
	echo "shim_path=$shim_path"
	echo "is_wsl=$is_wsl"
	echo "deps=$deps"
fi
