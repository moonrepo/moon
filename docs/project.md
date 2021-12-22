# Project

- [Configuration](#configuration)
  - [`project.yml`](#projectyml)
    - [project](#project)
    - [dependsOn](#dependson)
    - [fileGroups](#filegroups)
  - [`package.json`](#packagejson)
  - [`tsconfig.json`](#tsconfigjson)

## Configuration

All project configuration is located at the root of the project folder.

### `project.yml`

This configuration file _is not required_ but can be used to define additional metadata for a
project in the graph.

#### project

TODO

#### dependsOn

The optional `dependsOn` setting defines _other_ projects that _this_ project depends on, primarily
when generating the project and task graphs. The most common use case is building those projects
_before_ building this one. It will also sync [package.json](#packagejson) and
[tsconfig.json](#tsconfigjson) when applicable.

When defined, this setting requires an array of project IDs, which are the keys found in the
[`projects`](./workspace.md#projects) map.

```yaml
dependsOn:
	- 'dsl'
	- 'hooks'
```

#### fileGroups

> Knowledge of [`.monolith/project.yml`](./workspace.md#filegroups) is required before continuing.

As mentioned in the link above, file groups are a mechanism for grouping similar types of files
within a project using file glob patterns. By default, this setting _is not required_ for the
following reasons:

- File groups are an optional feature, and are designed for advanced use cases.
- File groups defined in `.monolith/project.yml` will be inherited by all projects.

The only scenario in which to define file groups at the project-level is when you want to _override_
file groups defined at the workspace-level.

For example, say we want to override the `sources` file group because our source folder is named
"lib" and not "src", we would define our `project.yml` as follows.

```yaml
fileGroups:
  sources:
    - 'lib/**/*'
    - 'types/**/*'
```

> File groups defined in `project.yml` will override file groups defined in `.monolith/project.yml`
> by object key, and _will not_ merge the value arrays.

### `package.json`

A Monolith project _does not require_ a `package.json`, but when one exists, the following
functionality is enabled.

- Dependency versions are included when computing cache keys.
- Depended on projects (`dependsOn`) are mapped as npm/pnpm/yarn workspace dependencies (when
  applicable).

### `tsconfig.json`

A Monolith project _does not require_ TypeScript or a `tsconfig.json`, but when one exists, the
following functionality is enabled.

- Depended on projects (`dependsOn`) are mapped as TypeScript project references (when applicable).
