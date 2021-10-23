# Monolith

Monolith is a Rust program for managing JavaScript based monorepo's.

## Terminology

- _Workspace_ - A repository that contains one or many projects. Is typically configured in the
  repository root.
- _Project_ - A library, application, package, etc, that contains source files, test files, assets,
  and more. Is declared with a `project.yml` file in the project root.
- _Toolchain_ - All technologies and languages that are required for running tasks within the
  workspace.
- _Task_ - A build, test, or action that can be ran within the context of a project.

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

### `.monolith/projects.yml`

Located at the workspace root, this file configures settings that are inherited by _all_ projects,
enabling easy reuse. These settings can be overriden at the project-level with their
`<project>/project.yml` file.

The following settings can be configured in this file:

- `fileGroups`

View the [`project.yml`](#projectyml) section for more information on this file structure.

### `.monolith/tasks.ts`

Located at the workspace root, this file configures tasks that are available to _all_ projects.
Workspace tasks can be overridden at the project-level with a `<project>/tasks.ts` file.

Tasks are declared by importing and executing a function, and then exporting the result. The name of
the export becomes the label in which to run the task on the command line.

```ts
import { setupBabel } from '@monolith/babel';
import { setupESLint } from '@monolith/eslint';
import { setupJest } from '@monolith/jest';

const extensions = ['.ts', '.tsx'];

// Run task with `mono build <project>:build`
// Will transpile `sources` to output directory
export const build = setupBabel({
	copyFiles: true,
	extensions,
	outputDir: 'lib',
});

// Run task with `mono test <project>:lint`
// Will lint `sources` and `tests` file groups
export const lint = setupESLint({ extensions });

// Run task with `mono test <project>:test`
// Will map `tests` file group to `--testMatch`
export const test = setupJest();
```

### `project.yml`

This files denotes a project, and must be located at the root of a project as defined by the
[`projects` setting](#monolithworkspaceyml).

```yaml
# Unique name of the project.
name: 'Example'

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

## Commands

### `mono build`

### `mono test`
