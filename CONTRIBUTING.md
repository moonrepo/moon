# Contributing

Contributions are always welcome, no matter how large or small!

## Prerequisites

- Node.js >= v14.15
- Rust >= 1.60

## Setup

On your first checkout of the repository, you'll need to install dependencies and build the project.

### Rust

Moon is built on Rust and requires `rustup` and `cargo` to exist in your environment. You can
[install Rust from the official website](https://www.rust-lang.org/tools/install).

We also require the following 3rd-party Cargo commands, which can be installed with the following.

```
cargo install --force cargo-make
cargo install --force cargo-insta
```

Once setup, we suggest building the Rust binary, as it's required for everything else.

```bash
cargo make build
```

### Node.js

Contributing to our `@moonrepo` npm packages requires Node.js and Yarn. We suggest
[installing Node.js with nvm](https://github.com/nvm-sh/nvm), and Yarn can later be installed with
`npm install -g yarn`.

Once setup, install dependencies build initial packages.

```bash
yarn install
yarn build
```

## How to

### Open development

All development on moon happens directly on GitHub. Both core team members and external contributors
send pull requests which go through the same review process.

### Branch organization

Submit all pull requests directly to the `master` branch. We only use separate branches for upcoming
releases / breaking changes, otherwise, everything points to master.

Code that lands in master must be compatible with the latest stable release. It may contain
additional features, but no breaking changes. We should be able to release a new minor version from
the tip of master at any time.

### Semantic versions

We utilize Yarn's [release workflow](https://yarnpkg.com/features/release-workflow) for declaring
version bumps and releasing packages. To enforce this standard, we have CI checks that will fail if
a package has been modified, but a version bump has not been declared.

### Reporting a bug

Please report bugs using the
[official issue template](https://github.com/moonrepo/moon/issues/new?assignees=&labels=bug&template=bug_report.md&title=),
only after you have previously searched for the issue and found no results. Be sure to be as
descriptive as possible and to include all applicable labels.

The best way to get your bug fixed is to provide a reduced test case. Please provide a public
repository with a runnable example, or a usable code snippet.

### Requesting new functionality

Before requesting new functionality, view the
[roadmap and backlog](https://github.com/moonrepo/moon/blob/master/ROADMAP.md) as your request may
already exist. If it does not exist, submit an
[issue using the official template](https://github.com/moonrepo/moon/issues/new?assignees=&labels=enhancement&template=feature_request.md&title=).
Be sure to be as descriptive as possible and to include all applicable labels.

### Submitting a pull request

We accept pull requests for all bugs, fixes, improvements, and new features. Before submitting a
pull request, be sure your build passes locally using the development workflow below.

## Development workflow

The following commands are available and should be used in your standard development workflow.

### Rust

To streamline development, we utilize [cargo-make](https://github.com/sagiegurari/cargo-make) for
common tasks.

- `cargo make build` - Builds all crates into a single `moon` binary.
- `cargo make format` - Formats code.
- `cargo make lint` - Runs the linter.
- `cargo make test` - Runs unit and integration tests.

### Node.js

This repo is powered by moon itself, which means that each npm package is a distinct moon project.
The list of projects can be found in [.moon/workspace.yml](./.moon/workspace.yml).

- `yarn moon run <project>:build` - Builds the package for distribution.
- `yarn moon run <project>:format` - Formats code.
- `yarn moon run <project>:lint` - Runs the linter.
- `yarn moon run <project>:test` - Runs unit tests.
- `yarn moon run <project>:typecheck` - Runs the type-checker.

Running all of these commands individually for _all_ packages is quite involved, so you can also
drop the project name to run the task in _all_ projects. For example: `yarn moon run :lint`

#### Type checking

Type checking is performed by [TypeScript](https://www.typescriptlang.org/). We prefer to run this
first, as valid typed code results in valid tests and lints.

#### Testing

Tests are written with [Jest](https://jestjs.io/). For every function or class, we expect an
associated `*.test.ts` test file in the package's tests folder. We also write unit tests, not
integration tests.

#### Linting

Linting is performed by [ESLint](https://eslint.org/). Most rules are errors, but those that are
warnings should _not_ be fixed, as they are informational. They primarily denote browser differences
and things that should be polyfilled.

#### Formatting

Code formatting is performed by [Prettier](https://prettier.io/). We prefer to run Prettier within
our code editors using `format-on-save` functionality.
