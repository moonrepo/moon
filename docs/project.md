# Projects

- [ID](#id)
- [Configuration](#configuration)
  - [`package.json`](#packagejson)
  - [`tsconfig.json`](#tsconfigjson)

A project is a library, application, package, binary, tool, etc, that contains source files, test
files, assets, resources, and more. A project must exist and be configured within a
[workspace](./workspace.md).

## ID

A project identifier, also knows an a PID, or simply ID, is a unique resource for locating a
project. The PID is explicitly configured within [`.moon/workspace.yml`](./workspace.md#projects),
as a key within the `projects` setting.

PIDs are used heavily by configuration and the command line to link and reference everything.
They're also a much easier concept for remembering projects than file system paths, and they
typically can be written with less key strokes.

## Configuration

All project configuration is located at the root of the project folder.

#### tasks

> Knowledge of [`.moon/project.yml`](./workspace.md#tasks) is required before continuing.

As mentioned in the link above, [tasks](./task.md) are actions that are ran within the context of a
project, and commonly wrap an npm or shell command. By default, this setting _is not required_ as
tasks are typically defined globally, and not all projects require tasks.

With that being said, projects can define tasks that are unique to themselves, and can also define
tasks that merge with global tasks of the same name!

```yaml
tasks:
  # Task unique to the project
  package:
    command: 'packemon'
    args:
      - 'build'
      - '--addExports'
  # Merge with a global task and provide additional args
  lint:
    args:
      - '--cache'
```

> Multiple [strategies](./task.md#merge-strategies) exist when merging tasks, so choose the one
> that's best for you!

### `package.json`

A Moon project _does not require_ a `package.json`, but when one exists, the following functionality
is enabled.

- Dependency versions are included when computing cache keys.
- Depended on projects (`dependsOn`) are mapped as npm/pnpm/yarn workspace dependencies (when
  applicable).

### `tsconfig.json`

A Moon project _does not require_ TypeScript or a `tsconfig.json`, but when one exists, the
following functionality is enabled.

- Depended on projects (`dependsOn`) are mapped as TypeScript project references (when applicable).

> File name can be customized with the `typescript.projectConfigFileName` setting.
