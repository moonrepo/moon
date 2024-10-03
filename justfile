set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

init:
	cargo install cargo-binstall
	cargo binstall cargo-insta cargo-nextest cargo-llvm-cov

# BUILDING

build:
	cargo build --workspace

build-wasm:
	cd wasm/test-plugin && cargo wasi build

# CHECKING

check:
	cargo check --workspace

format:
	cargo fmt --all -- --emit=files

format-check:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets

lint-fix:
	cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# TESTING

test $MOON_TEST="true" name="":
	cargo nextest run --workspace --config-file ./.cargo/nextest.toml {{name}}

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
