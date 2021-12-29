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
these tools "existing" in the current environment, as it ensures the following across any
environment or machine:

- The version and enabled features of a tool are identical.
- Tools are isolated and unaffected by external sources.
- Builds are consistent, reproducible, and _hopefully_ deterministic.

Furthermore, this avoids a developer, pipeline, machine, etc, having to pre-install all the
necessary tools, _and_ to keep them in sync as time passes.

## How it works

The toolchain is a `.moon` directory within the current user's home directory, e.g., `~/.moon`.

The first step in a tool's life-cycle is being downloaded to `~/.moon/temp`. Downloads are typically
an archive that can be unpacked into a target directory.

Once downloaded, we verify the downloaded file by running a sha256 checksum. If this check fails for
_any reason_, the toolchain is unusable, and the process is aborted.

After a successful verification, the last step in the tool's life-cycle can begin, installation.
Depending on the type of download, the installation process may differ. For archives, we unpack the
tool to `~/.moon/tools/<name>/<version>`.

## Configuration

The tools that are managed by the toolchain are configured through the
[`.moon/workspace.yml`](./workspace.md#workspaceyml) file.

## Supported tools

The following tools will be managed by the toolchain.

> Although the toolchain was designed for JavaScript projects, mainly powered by Node.js tooling, we
> may support other languages in the future when deemed necessary, like Ruby or Python.

### Node.js

Since Moon was designed for JavaScript based monorepo's, we intentionally support Node.js as a
first-class citizen within the toolchain. Because of this, Node.js is _always enabled_.

- Configured with: `node`
- Installed to: `~/.moon/tools/node/x.x.x`

### npm, npx

The `npm` and `npx` binaries come pre-installed with Node.js, and will _always exist_, regardless of
the `node.packageManager` setting.

- Configured with: `node.npm`
- Installed to: `~/.moon/tools/node/x.x.x/bin/npm` (and `npx`)

### pnpm

The [`pnpm`](https://pnpm.io) library can be used as an alternative package manager to npm, and will
be enabled when `node.packageManager` is set to "pnpm". The binary will be installed as a toolchain
global npm dependency.

- Configured with: `node.pnpm`
- Installed to: `~/.moon/tools/node/x.x.x/bin/pnpm`

### Yarn

The [`yarn`](https://yarnpkg.com) library can be used as an alternative package manager to npm, and
will be enabled when `node.packageManager` is set to "yarn". The binary will be installed as a
toolchain global npm dependency.

- Configured with: `node.yarn`
- Installed to: `~/.moon/tools/node/x.x.x/bin/yarn`

> Supports v1 and v2/v3 in `node-modules` or `pnp` linker mode.
