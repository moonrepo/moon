#!/usr/bin/env bash
# Copyright 2022 moonrepo, Inc.

# Thanks to Deno for the original script:
# https://github.com/denoland/deno_install/blob/master/install.sh

set -eo pipefail

bin="moon"
arch=$(uname -sm)
version="${1:-latest}"

if [[ "$OS" == "Windows_NT" ]]; then
	target="moon-x86_64-pc-windows-msvc.exe"
	bin="moon.exe"
else
	case "$arch" in
	"Darwin x86_64") target="moon-x86_64-apple-darwin" ;;
	"Darwin arm64") target="moon-aarch64-apple-darwin" ;;
	"Linux aarch64") target="moon-aarch64-unknown-linux" ;;
	"Linux x86_64") target="moon-x86_64-unknown-linux" ;;
	*)
		echo "Unsupported system or architecture \"$arch\". Unable to install moon!"
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
	download_url="https://github.com/moonrepo/moon/releases/latest/download/${target}"
else
	download_url="https://github.com/moonrepo/moon/releases/download/v${version}/${target}"
fi

if [ -z "$MOON_INSTALL_DIR" ]; then
	install_dir="$HOME/.moon/bin"
else
	install_dir="$MOON_INSTALL_DIR"
fi

bin_path="$install_dir/$bin"

if [ ! -d "$install_dir" ]; then
	mkdir -p "$install_dir"
fi

curl --fail --location --progress-bar --output "$bin_path" "$download_url"
chmod +x "$bin_path"

echo "Successfully installed moon to $bin_path"
echo "Manually update PATH in your shell to get started!"
echo
echo "  export PATH=\"$install_dir:\$PATH\""
echo
echo "Need help? Join our Discord https://discord.gg/qCh9MEynv2"

if [[ "$MOON_DEBUG" == "true" ]]; then
	echo
	echo "arch=$arch"
	echo "target=$target"
	echo "download_url=$download_url"
	echo "bin_path=$bin_path"
	echo "is_wsl=$is_wsl"
	echo "deps=$deps"
fi
