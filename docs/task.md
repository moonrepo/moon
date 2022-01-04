# Tasks

- [Tokens](#tokens)
  - [Variables](#variables)
  - [Functions](#functions)
    - [File groups](#file-groups)
- [Targets](#targets)
- [Merge strategies](#merge-strategies)

Tasks are commands that are ran in the context of a [project](./project.md). Underneath the hood, a
task is simply a node module binary or system/shell command that is ran as a child-process. Tasks
communicate between the Moon client and server through a JSON-like message system.

## Tokens

Tokens are variables and functions that can be used by `args`, `inputs`, and `outputs` when
configuring a task. They provide a way of accessing file group paths, referencing values from other
task fields, and referencing metadata about the project and task itself.

### Variables

TODO

### Functions

A token function is labeled as such as it takes a single argument, starts with an `@`, and is
formatted as `@name(arg)`. The following token functions are available, grouped by their
functionality.

> Token functions _must_ be the only content within a list item, as they expand to multiple file
> paths.

#### File groups

These functions reference file groups by name, where the name is passed as the argument.

##### `@dirs`

> Usable in `args` and `inputs`.

The `@dirs(file_group)` token will be replaced with an expanded list of directory paths, derived
from the file group of the same name. If a glob pattern is detected within the file group, it will
walk the file system and aggregate all directories found.

When used in `args`, it will return relative paths, while `inputs` will return absolute paths.

```yaml
fileGroups:
  lintable:
    - 'src'
    - 'tests'
    - 'scripts'

# Configured as
tasks:
  lint:
    command: 'eslint'
    args:
      - '@dirs(lintable)'
      - '--color'
    inputs:
      - '@dirs(lintable)'

# Resolves to
tasks:
  lint:
    command: 'eslint'
    args:
      - 'src'
      - 'tests'
      - 'scripts'
      - '--color'
    inputs:
      - '/path/to/project/src'
      - '/path/to/project/tests'
      - '/path/to/project/scripts'
```

##### `@files`

> Usable in `args` and `inputs`.

The `@files(file_group)` token will be replaced with an expanded list of file paths, derived from
the file group of the same name. If a glob pattern is detected within the file group, it will walk
the file system and aggregate all files found.

When used in `args`, it will return relative paths, while `inputs` will return absolute paths.

```yaml
fileGroups:
  config:
    - '*.config.js'
    - 'package.json'

# Configured as
tasks:
  build:
    command: 'webpack'
    args:
      - 'build'
      - '@files(config)'
    inputs:
      - '@files(config)'

# Resolves to
tasks:
  build:
    command: 'webpack'
    args:
      - 'build'
      - 'babel.config.js'
      - 'webpack.babel.js'
      - 'package.json'
    inputs:
      - '/path/to/project/babel.config.js'
      - '/path/to/project/webpack.babel.js'
      - '/path/to/project/package.json'
```

##### `@globs`

> Usable in `args` and `inputs`.

The `@globs(file_group)` token will be replaced with an expanded list of glob patterns (as-is),
derived from the file group of the same name. If a non-glob pattern is detected within the file
group, it will be ignored

When used in `args`, it will return relative paths, while `inputs` will return absolute paths _and_
also be used in affected files detection by matching against the patterns.

```yaml
fileGroups:
  tests:
    - 'tests/**/*'
    - '**/__tests__/**/*'

# Configured as
tasks:
  test:
    command: 'jest'
    args:
      - '--testMatch'
      - '@globs(tests)'
    inputs:
      - '@globs(config)'

# Resolves to
tasks:
  test:
    command: 'jest'
    args:
      - '--testMatch'
      - 'tests/**/*'
      - '**/__tests__/**/*'
    inputs:
      - '/path/to/project/tests/**/*'
      - '/path/to/project/**/__tests__/**/*'
```

##### `@root`

> Usable in `args` and `inputs`.

The `@root(file_group)` token will be replaced with the lowest common directory, derived from the
file group of the same name. If a glob pattern is detected within the file group, it will walk the
file system and aggregate all directories found before reducing.

When used in `args`, it will return relative paths, while `inputs` will return absolute paths.

```yaml
fileGroups:
  sources:
    - 'src/app'
    - 'src/packages'
    - 'src/scripts'

# Configured as
tasks:
  format:
    command: 'prettier'
    args:
      - '--write'
      - '@root(sources)'
    inputs:
      - '@root(sources)'

# Resolves to
tasks:
  format:
    command: 'prettier'
    args:
      - '--write'
      - 'src'
    inputs:
      - '/path/to/project/src'
```

> When there's no directies, or too many directories, this function will return the project root
> using `.`.

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
