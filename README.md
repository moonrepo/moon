# moon

> Currently a work in progress!

moon is a repository *m*anagement, *o*rganization, *o*rchestration, and *n*otification tool for
JavaScript based projects, written in Rust. Many of the concepts within moon are heavily inspired
from Bazel and other popular build systems, but tailored for the JavaScript ecosystem.

- [Documentation](./docs/README.md)

## Contributing

Moon is built on Rust and requires `rustup` and `cargo` to exist in your environment.  You can [install Rust from the official website](https://www.rust-lang.org/tools/install).

We also require additional Cargo commands, which can be installed with the following.

```
cargo install --force cargo-make
cargo install --force cargo-insta
```

To streamline development, we utilize [cargo-make](https://github.com/sagiegurari/cargo-make) for common tasks.

- `cargo make build` - Builds all crates into a single `moon` binary.
- `cargo make format` - Formats code.
- `cargo make lint` - Runs the linter (clippy).
- `cargo make test` - Runs unit and integration tests. Also sets up the moon toolchain.