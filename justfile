set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

init:
	cargo install cargo-binstall
	cargo binstall cargo-insta cargo-nextest cargo-llvm-cov

# BUILDING

build:
	cargo build --workspace

build-vcs-git:
	cd wasm && cargo build --package vcs_git --target wasm32-wasip1 --release
	cp wasm/target/wasm32-wasip1/release/vcs_git.wasm crates/vcs-plugin/res/vcs_git.wasm

build-wasm:
	cd wasm && cargo build --workspace --target wasm32-wasip1 --release
	cp wasm/target/wasm32-wasip1/release/vcs_git.wasm crates/vcs-plugin/res/vcs_git.wasm

# PROTOTYPES

# PROTOTYPE: Build and explore the source-control provider seam.
prototype-vcs:
	just build-wasm
	cargo run -p moon_vcs_plugin_prototype

# PROTOTYPE: Check Git and jj against Moon-level provider semantics.
prototype-vcs-conformance:
	just build-wasm
	cargo run -p moon_vcs_plugin_prototype -- --conformance

# PROTOTYPE: Measure provider boundary latency.
prototype-vcs-benchmark:
	just build-wasm
	cargo run --release -p moon_vcs_plugin_prototype -- --benchmark

# PROTOTYPE: Fail when release VCS plugin p95 latency exceeds its budget.
prototype-vcs-benchmark-check:
	just build-wasm
	cargo run --release -p moon_vcs_plugin_prototype -- --benchmark-check

# PROTOTYPE: Compare master and current Git-provider performance.
prototype-vcs-benchmark-comparison:
	bash "{{justfile_directory()}}/scripts/benchmark/vcsPlugin.sh"

# CHECKING

check:
	cargo check --workspace

check-wasm:
	cd wasm && cargo check --workspace

format:
	cargo fmt --all -- --emit=files

format-check:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

lint-fix:
	cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# TESTING

test $MOON_TEST="true" $STARBASE_TEST="true" filter="":
	cargo nextest run --workspace --no-fail-fast --config-file ./.cargo/nextest.toml {{filter}}

test-package name $MOON_TEST="true" $STARBASE_TEST="true":
	cargo nextest run --package {{name}} --no-fail-fast -j 4 --config-file ./.cargo/nextest.toml

test-ci $MOON_TEST="true" $STARBASE_TEST="true":
	cargo nextest run --workspace --config-file ./.cargo/nextest.toml --profile ci

# CODE COVERAGE

cov:
	cargo llvm-cov nextest --workspace --config-file ./.cargo/nextest.toml --profile ci

gen-report:
	cargo llvm-cov report --lcov --ignore-filename-regex error --output-path ./report.txt

gen-html:
	cargo llvm-cov report --html --ignore-filename-regex error --open

# RELEASING

release type="patch":
	bash ./scripts/release/tag.sh {{type}}

release-crates type="patch":
	cargo release {{type}} --execute -p moon_common -p moon_config -p moon_feature_flags -p moon_file_group -p moon_pdk -p moon_pdk_api -p moon_pdk_test_utils -p moon_project -p moon_target -p moon_task

bump:
	yarn version check --interactive

# OTHER

docs:
	cargo run -- run website:start

mcp:
	npx @modelcontextprotocol/inspector -- cargo run -- mcp

moon-check:
	cargo run -- check --all --log trace --summary

schemas:
	cargo run -p moon_config_schema --features typescript

clean-bazel-remote:
	rm -f ~/.moon/bazel-cache/cas.v2/.DS_Store && rm -f ~/.moon/bazel-cache/ac.v2/.DS_Store

bazel-remote:
	just clean-bazel-remote && bazel-remote --dir ~/.moon/bazel-cache --max_size 10 --storage_mode uncompressed --grpc_address 0.0.0.0:9092

bazel-remote-tls:
	just clean-bazel-remote && bazel-remote --dir ~/.moon/bazel-cache --max_size 10 --storage_mode uncompressed --grpc_address 0.0.0.0:9092 --tls_cert_file=./crates/remote/tests/__fixtures__/certs-local/server.crt --tls_key_file=./crates/remote/tests/__fixtures__/certs-local/server.key

bazel-remote-mtls:
	just clean-bazel-remote && bazel-remote --dir ~/.moon/bazel-cache --max_size 10 --storage_mode uncompressed --tls_cert_file=./crates/remote/tests/__fixtures__/certs-local/server.crt --tls_key_file=./crates/remote/tests/__fixtures__/certs-local/server.key --tls_ca_file=./crates/remote/tests/__fixtures__/certs-local/ca.crt
