# Workspace

- [Configuration](#configuration)
  - [`workspace.yml`](#workspaceyml)
    - [projects](#projects)
    - [node](#node)
      - [version](#version)
      - [packageManager](#packagemanager)
      - [npm, pnpm, yarn](#npm-pnpm-yarn)
        - [version](#version-1)
  - [`project.yml`](#projectyml)
    - [fileGroups](#filegroups)

A workspace is a directory that contains [projects](./project.md), manages a
[toolchain](./toolchain.md), and is typically coupled with a VCS repository. The root of a workspace
is denoted by a `.monolith` folder and a `package.json`.

By default Monolith has been designed for monorepos, but can also be used for polyrepos.

## Configuration

Configurations that apply to the entire workspace are located within a `.monolith` folder at the
workspace root.

> This folder _must_ be relative to the root `package.json` and it's associated lock file.

### `workspace.yml`

The `.monolith/workspace.yml` file configures projects and the toolchain.

#### projects

The `projects` setting is a map that defines the location of all [projects](./project.md) within the
workspace. Each project requires a unique ID as the map key, where this ID is used heavily on the
command line and within the project graph for uniquely identifying the project amongst all projects.
The map value is a file system path to the project folder, relative from the workspace root, and
must be contained within the workspace boundary.

```yaml
projects:
  admin: apps/admin
  web: apps/web
  dsl: packages/design-system
```

Unlike packages in the JavaScript ecosystem, a Monolith project _does not_ require a `package.json`.

> **Why doesn't Monolith auto-detect projects?** Monolith _does not_ automatically detect projects
> using file system globs for the following reasons:
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
> [active LTS version](https://nodejs.org/en/about/releases/) when not defined.

##### version

The `version` setting defines the explicit Node.js version to use. We require an explicit and
semantic major, minor, and patch version, to ensure the same environment is used across every
machine.

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

##### npm, pnpm, yarn

The `npm`, `pnpm`, and `yarn` settings are _optional_ fields for defining package manager specific
configuration. The chosen setting is dependent on the value of `node.packageManager`. If these
settings _are not defined_, the latest version of the active package manager will be used (when
applicable).

###### version

The `version` setting defines the explicit package manager version to use. We require an explicit
major, minor, and patch version, to ensure the same environment is used across every machine.

```yaml
node:
  yarn:
    version: '3.1.0'
```

### `project.yml`

The `.monolith/project.yml` file configures settings that are inherited by _every_ project in the
workspace. Projects can override these settings within their `<project path>/project.yml`
([view the projects docs for more information](./project.md#configuration)).

#### fileGroups

File groups are a mechanism for grouping similar types of files within a project using file glob
patterns. These groups are then used by tasks to calculate functionality like cache computation,
affected files since last change, command line arguments, deterministic builds, and more.

This setting requires a map, where the key is the file group name, and the value is a list of globs.
Globs are relative to a project -- even though these are defined globally. This enables enforcement
of organizational patterns across all projects in the workspace.

```yaml
fileGroups:
  configs:
    - '*.{js,json}'
  sources:
    - 'src/**/*'
    - 'types/**/*'
  tests:
    - 'tests/**/*.test.*'
    - '**/__tests__/**/*'
  assets:
    - 'assets/**/*'
    - 'images/**/*'
    - 'static/**/*'
    - '**/*.s?css'
    - '**/*.mdx?'
```

> The code snippet above is merely an example of file groups. Feel free to use those groups as-is,
> modify the glob lists, add and remove groups, or implement completely new groups. The choice is
> yours!
