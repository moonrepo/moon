These concise instructions help AI coding agents be productive in the moon monorepo.

Purpose
- Short orienting notes for contributors and AI assistants: what the repo is, its major parts,
  and the concrete commands and files we commonly interact with.

Quick start (common commands)
- Install JS workspace deps: `yarn` (this repo uses Yarn v4; Node >= 22.14.0)
- Build Rust workspace (default target is the CLI): `cargo build --workspace`
- Build only the CLI binary: `cargo build -p moon_cli --bin moon`
- Run the local CLI binary: `target/debug/moon <command>` (eg `target/debug/moon run :typecheck`)
- Run Rust tests (CI uses nextest): `cargo nextest run --workspace` or `nextest run --workspace`

High-level architecture (big picture)
- Rust-first monorepo: core implementation lives under `crates/` (CLI in `crates/cli`, app logic
  in `crates/app`, helpers in `crates/common`, etc.). See `Cargo.toml` workspace and
  `default-members = ["crates/cli"]` for defaults.
- JavaScript/TypeScript packages live under `packages/` (the `@moonrepo/cli` JS wrapper,
  `website`, utilities like `packages/runtime`, `packages/types`). The repo uses a Node
  workspace (see root `package.json` -> `workspaces`).
- Configuration for repo-scoped task orchestration is in `moon.yml` at the repo root and
  project-level `moon.yml` files under individual projects. Tasks often run toolchain
  binaries (see `.moon/toolchain.yml` referenced in docs).

Project-specific conventions and patterns
- Rust workspace: prefer path dependencies for intra-repo crates (see many `path = "../..."`
  entries in `crates/*/Cargo.toml`). Adding a crate normally involves adding it under
  `crates/` and letting the workspace glob pick it up.
- Node tooling: packages use `packemon`, `jest`, and a shared `tsconfig` layout. Types are
  cached into `.moon/cache/types/*` (see various `tsconfig.json` files).
- Tests and fixtures: Rust crate tests use `crates/*/tests` and per-crate fixtures (eg
  `crates/cli/tests/__fixtures__/**`). Look there for real-world usage examples used by
  unit/integration tests.

Integration points and notable dependencies
- Wasm / plugin runtime: `wasmtime`, `extism` and related crates are used for wasm-based
  plugins (see `crates/cli/Cargo.toml` and `wasm/`).
- Protobuf / plugin APIs: repo references `proto_*` crates and generated types used by the
  plugin system — edits to proto crates may require regenerating code in `proto/` (see
  `proto-plugin.toml` and `proto/` layout if present).
- Prebuilt runtime artifacts: `packages/core-*` directories contain prebuilt platform
  artifacts used for distribution/CI; treat them as platform-specific runtime bundles.

Testing & CI notes
- CI uses cargo-nextest and installs certain binaries via the `.moon` toolchain configuration
  (see docs under `website/docs/guides/rust`). Prefer `cargo nextest run --workspace` for
  fast, CI-aligned test runs.
- Many GH workflows live under `.github/workflows/` — check `rust.yml` for CI job steps and
  required tools (cargo-nextest, cargo-llvm-cov, etc.).

Where to look for examples (useful files)
- Root: `README.md`, `package.json`, `Cargo.toml`, `moon.yml`
- CLI entrypoint and binary building: `crates/cli/Cargo.toml`, `crates/cli/src/main.rs`
- Tests and fixtures: `crates/cli/tests/__fixtures__/**`, `crates/*/tests/**`
- JS packages: `packages/*/package.json` and `packages/*/README.md` (see `packages/cli`)
- Docs and rust guidance: `website/docs/` (handbook and toolchain examples)

Common gotchas / tips
- Node: use the repo Node engine (>=22.14.0) and Yarn v4; `yarn` bootstraps the workspace.
- Binary workflow: many local dev tasks expect the Rust binary to be built (see `package.json`
  scripts that reference `target/debug/moon`). If you see failing JS-side tasks, try building
  `moon` first.
- Cache & disk layout: `.moon/` is used for caches and generated artifacts and is gitignored.

If something here is missing or unclear, tell me which area (architecture, build, tests,
examples) you want expanded and I will iterate.
