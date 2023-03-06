#!/usr/bin/env bash
# Copyright 2022 moonrepo, Inc.

# Thanks to Deno for the original script:
# https://github.com/denoland/deno_install/blob/master/install.sh

set -e

bin="proto"
arch=$(uname -sm)
version="${1:-0.1.5}" # TODO
ext=".tar.xz"

if [ "$OS" = "Windows_NT" ]; then
	target="proto_cli-v$version-x86_64-pc-windows-msvc"
	bin="proto.exe"
	ext=".zip"
else
	case "$arch" in
	"Darwin x86_64") target="proto_cli-v$version-x86_64-apple-darwin" ;;
	"Darwin arm64") target="proto_cli-v$version-aarch64-apple-darwin" ;;
	# "Linux aarch64") target="proto_cli-v$version-aarch64-unknown-linux-gnu" ;;
	"Linux x86_64") target="proto_cli-v$version-x86_64-unknown-linux-gnu" ;;
	*)
		echo "Unsupported system or architecture \"$arch\". Unable to install proto!"
		exit 1
	 ;;
	esac
fi

if [[ "$arch" == "Linux"* ]]; then
	deps=$(ldd --version 2>&1 || true)

	if [[ $deps == *"musl"* ]]; then
		target="${target/gnu/musl}"
	fi
fi

wsl=$(uname -a)
if [[ "$wsl" == *"Microsoft"* || "$wsl" == *"microsoft"* ]]; then
  is_wsl=true
else
  is_wsl=false
fi

# if [ $# -eq 0 ]; then
# 	download_url="https://github.com/moonrepo/proto/releases/latest/download/$target$ext"
# else
# 	download_url="https://github.com/moonrepo/proto/releases/download/proto_cli-v$version/$target$ext"
# fi

download_url="https://github.com/moonrepo/proto/releases/download/proto_cli-v$version/$target$ext"
temp_dir="$HOME/.proto/temp/proto/$target"
download_file="$temp_dir$ext"
install_dir="$HOME/.proto/bin"
bin_path="$install_dir/$bin"

# Download and unpack in temp dir

if [ ! -d "$temp_dir" ]; then
	mkdir -p "$temp_dir"
fi

curl --fail --location --progress-bar --output "$download_file" "$download_url"
tar xf "$download_file" --strip-components 1 -C "$temp_dir"

# Move to bin dir and clean up

if [ ! -d "$install_dir" ]; then
	mkdir -p "$install_dir"
fi

mv "$temp_dir/$bin" "$bin_path"
chmod +x "$bin_path"
rm -rf "$download_file" "$temp_dir"

# Run setup script to update shells

$bin_path setup

echo "Successfully installed proto to $bin_path"
echo "Launch a new terminal window to start using proto!"
echo
echo "Need help? Join our Discord https://discord.gg/qCh9MEynv2"

if [ "$PROTO_TEST" = "true" ]; then
	echo
	echo "arch=$arch"
	echo "target=$target"
	echo "download_url=$download_url"
	echo "bin_path=$bin_path"
	echo "is_wsl=$is_wsl"
	echo "deps=$deps"
fi
