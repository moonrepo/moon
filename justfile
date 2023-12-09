export MOON_TEST := "true"

init:
	cargo install cargo-binstall
	cargo binstall cargo-insta cargo-nextest cargo-wasi

# BUILDING

build:
	cargo build --workspace

# CHECKING

format:
	cargo fmt --all -- --emit=files

format-check:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets

lint-fix:
	just lint --fix --allow-dirty --allow-staged

# TESTING

test name="":
	cargo nextest run --workspace --config-file ./.cargo/nextest.toml {{name}}

test-ci:
	just test --profile ci

# CODE COVERAGE

cov:
	cargo llvm-cov nextest --workspace --config-file ./.cargo/nextest.toml

cov-ci:
	just cov --profile ci

gen-report:
	cargo llvm-cov report --lcov --ignore-filename-regex error --output-path ./report.txt

gen-html:
	cargo llvm-cov report --html --ignore-filename-regex error --open

# OTHER

schemas:
	cargo run -p moon_config

gql:
	graphql-client introspect-schema http://localhost:8080/graphql --output nextgen/api/schema.json --header "X-Moonbase-TestingId: 1"
