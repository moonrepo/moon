# AGENTS.md

Guidance for AI coding agents working in the **moon** repository. moon is a repository management,
organization, orchestration, and notification tool for the web ecosystem. The core is written in
Rust and ships alongside a set of `@moonrepo/*` npm packages. moon is built _with_ moon, so every
npm package is itself a moon project.

## Repository layout

| Path         | Contents                                                            |
| ------------ | ------------------------------------------------------------------- |
| `.moon/`     | moon workspace config (`workspace.yml`, `toolchains.yml`, `tasks/`) |
| `crates/*`   | Rust crates; `crates/cli` is the default binary                     |
| `packages/*` | TypeScript npm packages; `@moonrepo/*` scope                        |
| `wasm/*`     | Test WASM plugins built for `wasm32-wasip1`                         |
| `website/`   | Documentation site (also a moon project)                            |

## Prerequisites

- Rust >= 1.96 (pinned in `rust-toolchain.toml`, edition 2024)
- Cargo — Rust toolchain
- Node.js >= 22.18
- Yarn >= 4
- [Just](https://github.com/casey/just) — Rust task runner
- [Vite+](https://viteplus.dev/) — JavaScript toolchain
- Git >= 2.28

First-time setup:

```bash
just init     # installs cargo-nextest, cargo-insta, cargo-llvm-cov, etc.
just build    # builds the `moon` binary (required for the JS workflow)
vp install    # installs node modules
```

## Rust workflow

Use `just` for all Rust tasks. **Always run format, lint, and test before finishing a change.**

- `just build` - Builds all Rust crates.
- `just check` - Checks all Rust crates without producing binaries.
- `just format` - Formats all Rust code with `rustfmt`.
- `just lint` - Lints all Rust code with `clippy` (treats warnings as errors).
- `just test` - Runs all Rust tests with `cargo nextest`.
- `just test <filter>` - Runs all Rust tests matching the filter.
- `just test-package` - Runs tests for a single package/crate by name.
- `just cov` - Runs Rust tests with LLVM coverage (slow).

### Conventions:

- **No warnings.** Clippy runs with `-D warnings`; treat every warning as an error.
- **No `std` hash collections.** `std::collections::HashMap`/`HashSet` are disallowed via
  `clippy.toml`. Use `rustc_hash::{FxHashMap, FxHashSet}` instead.
- **Snapshots** use [`insta`](https://insta.rs/); review changes with `cargo insta review`.
- **Tests** run under [nextest](https://nexte.st/). Tests must pass with `MOON_TEST=true` and
  `STARBASE_TEST=true` (the `just test` recipe sets these for you).
- Applicable crates are published independently using `cargo release`. Never automate this!

## JavaScript workflow

This repo is powered by moon, so each npm package under `packages/*` is a moon project. If the moon
binary has been built, the `yarn moon` command can be used, otherwise use `cargo run --`.

The following tasks can be run for each package:

- `{cmd} run <project>:build` - Builds the package.
- `{cmd} run <project>:test` - Runs unit tests.
- `{cmd} run <project>:typecheck` - Runs the type-checker.

While the following tasks are ran at the root level and apply to all packages:

- `{cmd} run root:format` - Formats all code.
- `{cmd} run root:lint` - Lints all code.

> Drop the project name to run a task across _all_ projects.

### Tooling and conventions:

- **Type checking** with [TypeScript](https://www.typescriptlang.org/) — run it first; valid types
  lead to valid tests and lints.
- **Testing** with [Vitest](https://vitest.dev/). Every function/class gets a sibling `*.test.ts` in
  the package's `tests/` folder. Write unit tests, not integration tests.
- **Linting** with [oxlint](https://oxc.rs/docs/guide/usage/linter). Errors must be fixed; **leave
  warnings alone** — they are informational (browser differences, polyfill hints), not actionable.
- **Formatting** with [oxfmt](https://oxc.rs/docs/guide/usage/formatter), ideally via format-on-save
  in your editor. Indentation is **tabs**.

## Pull requests

- Target `master` for bug fixes; target `develop-x.x` for new features / breaking changes.
- Code on `master` must stay compatible with the latest stable release (no breaking changes).
- npm package version bumps use Yarn's
  [release workflow](https://yarnpkg.com/features/release-workflow) — CI fails if a package was
  modified without a declared bump.
- Make sure the build passes locally (format, lint, test for both Rust and JS) before opening a PR.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for the full contributor guide.
