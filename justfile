set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

init:
	cargo install cargo-binstall
	cargo binstall cargo-insta cargo-nextest cargo-llvm-cov

# BUILDING

build:
	cargo build --workspace

build-wasm:
	cd wasm && cargo build --workspace --target wasm32-wasip1 --release

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

test $MOON_TEST="true" name="":
	cargo nextest run --workspace --no-fail-fast --config-file ./.cargo/nextest.toml {{name}}

test-ci $MOON_TEST="true":
	cargo nextest run --workspace --config-file ./.cargo/nextest.toml --profile ci

# CODE COVERAGE

cov:
	cargo llvm-cov nextest --workspace --config-file ./.cargo/nextest.toml --profile ci

gen-report:
	cargo llvm-cov report --lcov --ignore-filename-regex error --output-path ./report.txt

gen-html:
	cargo llvm-cov report --html --ignore-filename-regex error --open

# RELEASING

bump type="patch":
	bash ./scripts/version/bumpBinaryVersions.sh {{type}}

bump-all:
	bash ./scripts/version/forceBumpAllVersions.sh

bump-interactive:
	yarn version check --interactive

release:
	node ./scripts/version/applyAndTagVersions.mjs

# OTHER

docs:
	cargo run -- run website:start

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
