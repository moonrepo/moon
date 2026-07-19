#!/usr/bin/env bash

# PROTOTYPE: Build master and the current revision, then compare identical
# Git-only workloads before measuring the WASM-to-jj path separately.

set -euo pipefail

root="$(jj root)"
temp_root="$(mktemp -d "${TMPDIR:-/tmp}/moon-vcs-benchmark.XXXXXX")"
workspace_name="moon-vcs-benchmark-master-$$"
master_workspace="$root/target/vcs-benchmark/master-source"
master_fixture="$temp_root/master-fixture"
current_fixture="$temp_root/current-fixture"
base_fixture="$temp_root/base-fixture"
master_target="$root/target/vcs-benchmark/master"
current_target="$root/target/vcs-benchmark/current"

cleanup() {
	jj workspace forget "$workspace_name" >/dev/null 2>&1 || true
	rm -rf "$master_workspace"
	rm -rf "$temp_root"
}
trap cleanup EXIT

mkdir -p "$(dirname "$master_workspace")" "$temp_root/moon-home"
rm -rf "$master_workspace"
export MOON_HOME="$temp_root/moon-home"

if [[ "${MOON_VCS_BENCH_SKIP_BUILD:-0}" != "1" ]]; then
	jj workspace add --name "$workspace_name" --revision master "$master_workspace" >/dev/null

	echo "Building master Moon..."
	CARGO_TARGET_DIR="$master_target" cargo build \
		--manifest-path "$master_workspace/Cargo.toml" \
		--release \
		-p moon_cli \
		--bin moon

	echo "Building current Moon and benchmark driver..."
	cargo build --manifest-path wasm/Cargo.toml -p vcs_git --target wasm32-wasip1 --release
	mkdir -p crates/vcs-plugin/res
	cp wasm/target/wasm32-wasip1/release/vcs_git.wasm crates/vcs-plugin/res/vcs_git.wasm
	CARGO_TARGET_DIR="$current_target" cargo build --release -p moon_cli --bin moon
	cargo build --release -p moon_vcs_plugin_prototype
	cargo build --manifest-path wasm/Cargo.toml -p vcs_jj_prototype --target wasm32-wasip1 --release
fi

mkdir -p "$base_fixture/.moon" "$base_fixture/app"
cat > "$base_fixture/.moon/workspace.yml" <<'YAML'
projects:
  app: app
vcs:
  defaultBranch: master
YAML
cat > "$base_fixture/app/moon.yml" <<'YAML'
language: javascript
tasks: {}
YAML
printf 'initial\n' > "$base_fixture/app/input.txt"
printf '.moon/cache/\n' > "$base_fixture/.gitignore"

git init --quiet --initial-branch master "$base_fixture"
git -C "$base_fixture" add .
git -C "$base_fixture" -c user.name=Moon -c user.email=moon@example.com \
	commit --quiet -m initial
git -C "$base_fixture" branch base
printf 'committed\n' > "$base_fixture/app/committed.txt"
git -C "$base_fixture" add .
git -C "$base_fixture" -c user.name=Moon -c user.email=moon@example.com \
	commit --quiet -m second

git clone --quiet "$base_fixture" "$master_fixture"
git clone --quiet "$base_fixture" "$current_fixture"
git -C "$master_fixture" branch base origin/base >/dev/null
git -C "$current_fixture" branch base origin/base >/dev/null
printf 'working\n' >> "$master_fixture/app/input.txt"
printf 'untracked\n' > "$master_fixture/app/new.txt"
printf 'working\n' >> "$current_fixture/app/input.txt"
printf 'untracked\n' > "$current_fixture/app/new.txt"

"$root/target/release/moon_vcs_plugin_prototype" \
	--benchmark-git-comparison \
	"$master_target/release/moon" \
	"$current_target/release/moon" \
	"$master_fixture" \
	"$current_fixture"

"$root/target/release/moon_vcs_plugin_prototype" --benchmark
