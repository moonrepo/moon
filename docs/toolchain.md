# Toolchain

- [How it works](#how-it-works)
- [Configuration](#configuration)
- [Supported tools](#supported-tools)
  - [Node.js](#nodejs)
  - [npm, npx](#npm-npx)
  - [pnpm](#pnpm)
  - [Yarn](#yarn)

The toolchain is an internal layer for downloading, installing, and managing tools (languages,
libraries, and binaries) that are required at runtime. We embrace this approach over relying on
these tools "existing", as it ensures the following across all environments and machines:

- Avoids a developer or pipeline having to pre-install all the necessary tools.
- The version and enabled features of a tool are identical.
- And lastly, builds are consistent, reproducible, and hopefully deterministic.

## How it works

TODO

## Configuration

The tools that are managed by the toolchain are configured through the
[`.monolith/workspace.yml`](./workspace.md#workspaceyml) file.

## Supported tools

The following tools will be managed by the toolchain.

> Although the toolchain was designed for JavaScript projects, mainly powered by Node.js tooling, we
> may support other languages in the future when deemed necessary, like Ruby or Python.

### Node.js

Since Monolith was designed for JavaScript based monorepo's, we intentionally support Node.js as a
first-class citizen within the toolchain.

- Configured with: `node`
- Installed to: `~/.monolith/tools/node/x.x.x`

### npm, npx

The `npm` and `npx` binaries come pre-installed with Node.js, and will _always exist_, regardless of
the `node.packageManager` setting.

- Configured with: `node.npm`
- Installed to: `~/.monolith/tools/node/x.x.x/bin/npm` (and `npx`)

### pnpm

The [`pnpm`](https://pnpm.io) library can be used as an alternative package manager to `npm`, and
will be enabled when `node.packageManager` is set to "pnpm". The binary will be installed as a
global toolchain npm dependency.

- Configured with: `node.pnpm`
- Installed to: `~/.monolith/tools/node/x.x.x/bin/pnpm`

### Yarn

The [`yarn`](https://yarnpkg.com) library can be used as an alternative package manager to `npm`,
and will be enabled when `node.packageManager` is set to "yarn". The binary will be installed as a
global toolchain npm dependency.

- Configured with: `node.yarn`
- Installed to: `~/.monolith/tools/node/x.x.x/bin/yarn`

> Supports both Yarn v1 and Yarn v2/v3 in `node-modules` and `pnp` linker mode.
