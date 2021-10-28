# Monolith

Monolith is a Rust program for managing JavaScript based monorepo's.

> Inspired heavily from Bazel.

## Concepts

### Workspace

A workspace is a VCS repository that contains one or many [projects](#project). Although monolith
has been designed for monorepos, it can be used for polyrepos.

The root of a workspace is defined by a `.monolith` folder. A repository may contain multiple
workspaces as long as they are configured in separate and distinct folder trees.

### Toolchain

All technologies, languages, libraries, etc that are required for running [tasks](#task) within a
[workspace](#workspace). The toolchain _must_ be unaffected by external sources and _must_ be
deterministic between machines.

### Project

A project is a library, application, package, binary, etc, that contains source files, test files,
assets, resources, and more. A project is denoted with a [`project.yml`](#projectyml) file, and must
exist within a [workspace](#workspace).

#### PID

A PID, or project identifier, is a unique resource for locating and referencing a project. The PID
is explicitly derived from the workspace file system structure, and is composed by the folder path
from the workspace root to the project root. Let's demonstrate this with the an example structure.

```
.monolith/
apps/
  client/
    project.yml
  server/
    project.yml
packages/
  design/
    system/
      project.yml
  data/
    project.yml
```

In the example above, the following would be valid PIDs: `/apps/client`, `/apps/server`,
`/packages/design/system`, `/packages/data`. The leading forward slash (`/`) is used as a
designation for "starting from the workspace root".

Because of this, PIDs _may be relative_ from the current working folder. For example, if you're in
the `packages/design` folder, you may run a task with `bazel test system:lint` instead of the
fully-qualified `bazel test /packages/design/system:lint`.

### Tasks

An action that can be ran within the context of a [project](#project), and are configured through a
[`tasks.ts`](#tasksts) file. Is separated into the following types:

- **Build** - Generates an output from an input. Example: babel, rollup, webpack.
- **Test** - Validates criteria on a set of inputs. Example: jest, eslint, typescript.
- **Run** - Runs a one-off or long-lived process. Example: (watch mode), prettier, ts-node.

## Configuration

All Monolith configuration files are written in YAML or TypeScript, depending on the type of file.

- **Why YAML instead of JSON?** JSON is a data format by design, not a configuration format. Because
  of this, and its lack of comments, YAML is a better option.
- **Why TypeScript?** Since Monolith is designed for JavaScript based projects and tooling, it makes
  sense to also configure in a JavaScript-like language. This choice has the added benefit of these
  files also being linted, type checked, and statically analyzed like the rest of the workspace.

### `.monolith/workspace.yml`

Located at the workspace root, this file configures the developer and runtime environment for the
toolchain. This includes the Node.js version, a chosen package manager, and the package manager
version, project locations, and more.

```yaml
# The Node.js version to install and configure within the toolchain.
node:
  version: '14.18.1'
  packageManager: 'yarn'
  shasums:
    windows: '86737cd4544c4f8cda2abd8e60709a87dbf46119062c5f1d4ec297f71a9e204b'
    macos: '78731152378577decf681167f4c6be6c31134dfef07403c1cebfbd3289d3886f'
    linux: '3fcd1c6c008c2dfddea60ede3c735696982fb038288e45c2d35ef6b2098c8220'

# Yarn version to install and configure within the toolchain.
# Only used when `packageManager` is yarn. Other fields `npm` and `pnpm` also exist.
yarn:
  version: '3.1.0' # or source

# File system patterns to locate `project.yml` files.
# These patterns help to enforce a specific file system structure.
projects:
  - 'apps/*/project.yml'
  - 'packages/*/project.yml'
```

### `.monolith/project.yml`

Located at the workspace root, this file configures settings that are inherited by _all_ projects,
enabling reuse and enforcing patterns. However, these settings can be overriden at the project-level
with their [`<pid>/project.yml`](#projectyml) file.

The following settings can be configured in this file:

- `fileGroups`

### `.monolith/tasks.ts`

Located at the workspace root, this file configures tasks that are available to _all_ projects.
Workspace tasks can be overridden at the project-level with a [`<pid>/tasks.ts`](#tasksts) file.

### `project.yml`

This files denotes a project, and must be located at the root of a project as defined by the
[`projects` setting](#monolithworkspaceyml).

```yaml
# Unique name of the project.
name: 'Example'

# Description of the project.
description: 'A description of what the example project does.'

# The team or organization that owns and maintains the project.
# Can be a title, LDAP name, GitHub team, etc.
owner: 'infra'

# The Slack/Teams/Discord/etc channel to discuss the project.
channel: '#infra'

# File system patterns relative to the project root for grouping
# files based on their use case. These groups are then used by
# tasks to calculate functionality like cache hit/miss, affected
# files since last change, hot reloading, deterministic builds, etc.
fileGroups:
  # List of non-test JS/TS source files.
  # This may include runtime, published, or distributable files.
  sources:
    - 'src/**/*.{ts,tsx}'

  # List of non-source JS/TS test files.
  # This may include unit tests, E2E tests, stories, etc.
  tests:
    - 'tests/**/*.test.{ts,tsx}'
    - '**/__tests__/**/*'

  # Static assets within the project.
  # This may include styles, images, videos, etc.
  assets:
    - 'assets/**/*'
    - 'src/**/*.css'
    - '**/*.md'

  # Runtime required resources, that are not JS/TS.
  # This may include i18n translations, binaries, etc.
  resources:
    - 'messages/**/*.json'

  # Configuration files for project-level tooling.
  configs:
    - '*.config.js'
    - '*.json'
```

### `tasks.ts`

Tasks are declared by importing and executing a function, and then exporting the result. The name of
the export becomes the label in which to run the task on the command line.

```ts
import { setupBabel } from '@monolith/babel';
import { setupESLint } from '@monolith/eslint';
import { setupJest } from '@monolith/jest';

const extensions = ['.ts', '.tsx'];

// Run task with `mono build <pid>:build`
// Will transpile `sources` to output directory
export const build = setupBabel({
	copyFiles: true,
	extensions,
	outputDir: 'lib',
});

// Run task with `mono test <pid>:lint`
// Will lint `sources` and `tests` file groups
export const lint = setupESLint({ extensions });

// Run task with `mono test <pid>:test`
// Will map `tests` file group to `--testMatch`
export const test = setupJest();
```

## Commands

### `mono build`

### `mono test`

### `mono run`
