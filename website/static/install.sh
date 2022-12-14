#!/usr/bin/env bash
# Copyright 2022 moonrepo LLC

# Thanks to Deno for the original script:
# https://github.com/denoland/deno_install/blob/master/install.sh

set -e

bin="moon"
arch=$(uname -sm)
version="${1:-latest}"

if [ "$OS" = "Windows_NT" ]; then
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

if [ $# -eq 0 ]; then
	download_url="https://github.com/moonrepo/moon/releases/latest/download/${target}"
else
	download_url="https://github.com/moonrepo/moon/releases/download/%40moonrepo%2Fcli%40${1}/${target}"
fi

install_dir="$HOME/.moon/tools/moon/$version"
bin_path="$install_dir/$bin"

if [ ! -d "$install_dir" ]; then
	mkdir -p "$install_dir"
fi

curl --fail --location --progress-bar --output "$bin_path" "$download_url"
chmod +x "$bin_path"
ln -sf "$bin_path" "/usr/local/bin/$bin"

echo "Successfully installed moon to $bin_path"
echo "Run 'moon --help' to get started!"
echo
echo "Need help? Join our Discord https://discord.gg/qCh9MEynv2"

if [ "$MOON_TEST" = "true" ]; then
	echo
	echo "arch=$arch"
	echo "target=$target"
	echo "download_url=$download_url"
	echo "bin_path=$bin_path"
	echo "deps=$deps"
fi
