# Workspace

TODO

## Configuration

Configurations that apply to the entire workspace are located within a `.monolith` folder at the
root of the workspace.

> This folder _must_ be relative to the root `package.json` and package manager lock file.

### workspace.yml

The `.monolith/workspace.yml` file configures settings for the toolchain runtime and project
locations.

#### projects

The `projects` setting is a map that defines the location of all projects within the workspace. Each
project requires a label as the map key, where this label is used heavily on the command line and
within the project graph for uniquely identifying the project amongst all projects. The map value is
a file path to the project folder, relative from the workspace root.

```yaml
projects:
	admin: apps/admin
	web: apps/web
	ds: packages/design-system
```

Unlike packages in the JavaScript ecosystem, a Monolith project _does not_ require a `package.json`.

> **Why doesn't Monolith auto-detect projects?**
>
> Monolith _does not_ automatically detect projects using file system globs for the following
> reasons:
>
> - Depth-first scans are expensive, especially when the workspace continues to grow.
> - CI and other machines may inadvertently detect more projects because of left over artifacts.
> - Centralizing a manifest of projects allows for an easy review and approval process.

#### node

The `node` setting defines the Node.js version and package manager to install within the toolchain,
as Monolith _does not_ use a Node.js binary found on the host machine. Managing the Node.js version
within the toolchain ensures a deterministic environment across any machine (whether a developer,
CI, or production machine).

> This setting is optional, and will default Node.js to the latest
> [current LTS version](https://nodejs.org/en/about/releases/) when not defined.

##### version

The `version` setting defines the explicit Node.js version to use. We require an explicit major,
minor, and patch version, to ensure the same environment is used across every machine.

```yaml
node:
	version: '16.13.0'
```

##### packageManager

This setting defines which package manager to utilize within the workspace. Supports `npm`
(default), `pnpm`, or `yarn`.

```yaml
node:
	packageManager: yarn
```

#### npm, pnpm, yarn

The `npm`, `pnpm`, and `yarn` settings are _optional_ fields for defining package manager specific
configuration. The chosen setting is dependent on the value of `node.packageManager`. If these
settings _are not defined_, the latest version of the active package manager will be used.

##### version

The `version` setting defines the explicit package manager version to use. We require an explicit
major, minor, and patch version, to ensure the same environment is used across every machine.

```yaml
yarn:
	version: '3.1.0'
```
