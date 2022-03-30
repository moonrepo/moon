# Projects

- [ID](#id)
- [Configuration](#configuration)
  - [`project.yml`](#projectyml)
    - [project](#project)
      - [type](#type)
      - [name](#name)
      - [description](#description)
      - [channel](#channel)
      - [owner](#owner)
      - [maintainers](#maintainers)
    - [dependsOn](#dependson)
    - [fileGroups](#filegroups)
    - [tasks](#tasks)
    - [workspace](#workspace)
      - [inheritedTasks](#inheritedTasks)
        - [exclude](#exclude)
        - [include](#include)
        - [rename](#rename)
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

### `project.yml`

This configuration file _is not required_ but can be used to define additional metadata for a
project in the graph.

#### project

The optional `project` setting defines metadata about the project itself. Although this setting is
optional, when defined, all fields within it _must_ be defined as well.

```yaml
project:
  type: 'library'
  name: 'Moon'
  description: 'A monorepo management tool.'
  channel: '#moon'
  owner: 'infra'
  maintainers: ['miles.johnson']
```

The information listed within `project` is purely informational and primarily displayed within the
CLI. However, this setting exists for you, your team, and your company, as a means to identify and
organize all projects. Feel free to build your own tooling around these settings!

##### type

The type of project. Supports the following values:

- `application` - A backend or frontend application that communicates over HTTP, TCP, RPC, etc.
- `library` - A self-contained, shareable, and publishable set of code.
- `tool` - An internal tool, command line program, one-off script, etc.

##### name

A human readable name of the project. This is _different_ from the unique project ID.

##### description

A description of what the project does and aims to achieve. Be as descriptive as possible, as this
is the kind of information search engines would index on.

##### channel

The Slack, Discord, Teams, IRC, etc channel name (with leading #) in which to discuss the project.

##### owner

The team or organization that owns the project. Can be a title, LDAP name, GitHub team, etc. We
suggest _not_ listing people/developers as the owner, use [maintainers](#maintainers) instead.

##### maintainers

A list of people/developers that maintain the project, review code changes, and can provide support.
Can be a name, email, LDAP/GitHub username, etc, the choice is yours.

#### dependsOn

The optional `dependsOn` setting defines _other_ projects that _this_ project depends on, primarily
when generating the project and dependency graphs. The most common use case is building those
projects _before_ building this one. It will also sync [package.json](#packagejson) and
[tsconfig.json](#tsconfigjson) when applicable.

When defined, this setting requires an array of project IDs, which are the keys found in the
[`projects`](./workspace.md#projects) map.

```yaml
dependsOn:
  - 'dsl'
  - 'hooks'
```

#### fileGroups

> Knowledge of [`.moon/project.yml`](./workspace.md#filegroups) is required before continuing.

As mentioned in the link above, file groups are a mechanism for grouping similar types of files
within a project using file glob patterns. By default, this setting _is not required_ for the
following reasons:

- File groups are an optional feature, and are designed for advanced use cases.
- File groups defined in `.moon/project.yml` will be inherited by all projects.

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

> File groups defined in `project.yml` will override file groups defined in `.moon/project.yml` by
> object key, and _will not_ merge the value arrays.

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

#### workspace

The optional `workspace` setting dictates how a project interacts with settings at the
workspace-level.

##### inheritedTasks

Provides a layer of control when inheriting tasks from the
[global project config](./workspace.md#projectyml).

###### exclude

The optional `exclude` setting permits a project to exclude specific tasks from being inherited. It
accepts a list of strings, where each string is the ID of a global task to exclude.

```yaml
workspace:
  inheritedTasks:
    # Exclude the inherited `test` task for this project
    exclude: ['test']
```

> Exclusion is applied after inclusion and before renaming.

###### include

The optional `include` setting permits a project to _only_ include specific inherited tasks (works
like an allow/white list). It accepts a list of strings, where each string is the ID of a global
task to include.

When this field is not defined, the project will inherit all tasks from the global project config.

```yaml
workspace:
  inheritedTasks:
    # Include *no* tasks (works like a full exclude)
    include: []

    # Only include the `lint` and `test` tasks for this project
    include:
      - 'lint'
      - 'test'
```

> Inclusion is applied before exclusion and renaming.

###### rename

The optional `rename` setting permits a project to rename the inherited task within the current
project. It accepts a map of strings, where the key is the original ID (found in the global project
config), and the value is the new ID to use.

For example, say we have 2 tasks in the global project config called `buildPackage` and
`buildApplication`, but we only need 1, and since we're an application, we should omit and rename.

```yaml
workspace:
  inheritedTasks:
    exclude: ['buildPackage']
    rename:
      buildApplication: 'build'
```

> Renaming occurs after inclusion and exclusion.

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
