#!/usr/bin/env bash

echo "OS: $(uname)"
echo "Arch: $(uname -m)"

function getBinaryName() {
	if [[ -z "${BINARY}" ]]; then
		case "$(uname -s)" in
			CYGWIN*|MINGW32*|MSYS*|MINGW*)
				echo -n "moon.exe"
				;;

			*)
				echo -n "moon"
				;;
		esac
	else
		echo -n "$BINARY"
	fi
}

function getPackageFromTarget() {
	case "$1" in
		aarch64-apple-darwin)
			echo -n "core-macos-arm64"
			;;

		aarch64-unknown-linux-gnu)
			echo -n "core-linux-arm64-gnu"
			;;

		aarch64-unknown-linux-musl)
			echo -n "core-linux-arm64-musl"
			;;

		x86_64-apple-darwin)
			echo -n "core-macos-x64"
			;;

		x86_64-pc-windows-msvc)
			echo -n "core-windows-x64-msvc"
			;;

		x86_64-unknown-linux-musl)
			echo -n "core-linux-x64-musl"
			;;

		*)
			echo -n "core-linux-x64-gnu"
			;;
	esac
}
