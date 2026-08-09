# Nix flake

`flake.nix` at the repository root lets Nix users build and run moon straight from a git ref, and
gives contributors on NixOS a dev shell with the right toolchain. It is independent of the
[nixpkgs `moon` package](https://search.nixos.org/packages?query=moon), which is maintained by
nixpkgs contributors on their own schedule.

Outputs, for `x86_64-linux`, `aarch64-linux`, and `aarch64-darwin`:

- `packages.default` and `packages.moon` build the `moon` and `moonx` binaries with shell
  completions.
- `packages.moon-deps` exposes the compiled Cargo dependencies for cache warming.
- `apps.default` runs the `moon` binary.
- `devShells.default` provides the Rust toolchain, `just`, and `cargo-nextest`.

## Releases

Nothing to do. `version` is read from `crates/cli/Cargo.toml`, so a version bump flows into the
package and the `meta.changelog` link without touching `flake.nix`.

## Updating inputs

The flake pins nixpkgs, flake-utils, rust-overlay, and Crane in `flake.lock`. To move them forward:

```shell
nix flake update
nix flake update nixpkgs # or a single input
```

Commit the resulting `flake.lock`. CI runs `nix flake check --no-write-lock-file`, so a stale or
missing lockfile fails the job instead of being rewritten silently.

## Rust toolchain

`rust-toolchain.toml` is the source of truth. rust-overlay reads it, so bumping the channel there
bumps the flake too.

The dev shell adds the `rust-src` component on top, because rust-analyzer needs it and the `default`
profile leaves it out. The package build deliberately skips it, since the compiler sources add
gigabytes to the closure.

## Build dependencies

`protobuf` is a native build input because `crates/daemon-proto/build.rs` runs `protoc` at compile
time. `openssl` plus `OPENSSL_NO_VENDOR` make reqwest link against the system library rather than
compiling a vendored copy, which the `native-tls-vendored` feature would otherwise do.

Crane vendors Cargo dependencies from `Cargo.lock`. If moon ever takes a dependency from a git
source, Crane will vendor it from the locked revision.

Crane compiles dependencies separately from the final package so application changes can reuse them.
The package source is restricted to Cargo sources and compile-time assets, preventing unrelated files
and version control metadata from invalidating the build.

Tests run with `doCheck = false`. They download Node.js, Bun, Deno, and other toolchains at runtime,
which the Nix sandbox blocks.

## CI

`.github/workflows/nix.yml` runs `nix flake check` and builds the package for every supported Linux
and macOS system when `flake.nix`, `flake.lock`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, or
anything under `crates/` changes. All jobs pull from the public `moonrepo` Cachix cache. Trusted push
and manual runs publish new paths when the repository has a `CACHIX_AUTH_TOKEN` secret; pull requests
remain read-only.

The flake advertises the cache through `nixConfig`. Pass `--accept-flake-config` when running Nix
non-interactively, or configure the cache with `cachix use moonrepo`.

Before pushing a change to the flake, run the same two commands locally:

```shell
nix flake check --accept-flake-config
nix build .#default --accept-flake-config
```
