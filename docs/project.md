# Project

- [Configuration](#configuration)
  - [`project.yml`](#projectyml)
    - [project](#project)
    - [dependsOn](#dependson)
    - [fileGroups](#filegroups)
  - [`package.json`](#packagejson)

## Configuration

### `project.yml`

#### project

#### dependsOn

#### fileGroups

> Knowledge of [`.monolith/project.yml`](./workspace.md#filegroups) is required before continuing.

As mentioned in the link above, file groups are a mechanism for grouping similar types of files
within a project using file glob patterns. By default, this field _is not required_ for the
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
