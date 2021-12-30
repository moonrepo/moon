# Tasks

- [Targets](#targets)
- [Merge strategies](#merge-strategies)

Tasks are commands that are ran in the context of a [project](./project.md). Underneath the hood, a
task is simply a node module binary or system/shell command that is ran as a child-process. Tasks
communicate between the Moon client and server through a JSON-like message system.

## Targets

A target is an identifier that pairs a project to an owned task, in the format of
"project_id:task_id". Targets are used by terminal commands...

```shell
$ moon run project:build
```

And task configurations for declaring cross-project or cross-task dependencies.

```yaml
tasks:
  build:
    command: 'webpack'
    deps:
      - 'dsl:build'
```

## Merge strategies

When a [global task](./workspace.md#tasks) and [local task](./project.md#tasks) of the same name
exist, they are merged into a single task. To accomplish this, one of many
[merge strategies](./workspace.md#options) can be used.

Merging is applied to the list parameters `args`, `deps`, `inputs`, and `outputs`, using the
`mergeArgs`, `mergeDeps`, `mergeInputs` and `mergeOutputs` options respectively. Each of these
options support one of the following strategy values.

- `append` (default) - List items found in the local task are merged _after_ the items found in the
  global task. For example, this strategy is useful for toggling flag arguments.
- `prepend` - List items found in the local task are merged _before_ the items found in the global
  task. For example, this strategy is useful for applying option argument that must come before
  positional arguments.
- `replace` - The list found in the local task entirely _replaces_ the list found in the global
  task. This strategy is useful when you need full control.

All 3 of these strategies are demonstrated below, with a somewhat contrived example, but you get the
point.

```yaml
# Global
tasks:
  build:
    command: 'webpack'
    args:
      - '--mode'
      - 'production'
      - '--color'
    deps:
      - 'dsl:build'
    inputs:
      - '/webpack.config.js'
    outputs:
      - 'build/'

# Local
tasks:
  build:
    args:
      - '--no-color'
      - '--no-stats'
    deps:
      - 'hooks:build'
    inputs:
      - 'webpack.config.js'
    options:
      mergeArgs: 'append'
      mergeDeps: 'prepend'
      mergeInputs: 'replace'

# Merged result
tasks:
  build:
    command: 'webpack'
    args:
      - '--mode'
      - 'production'
      - '--color'
      - '--no-color'
      - '--no-stats'
    deps:
      - 'hooks:build'
      - 'dsl:build'
    inputs:
      - 'webpack.config.js'
    options:
      mergeArgs: 'append'
      mergeDeps: 'prepend'
      mergeInputs: 'replace'
```
