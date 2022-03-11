# Workspace

- [Configuration](#configuration)
  - [`workspace.yml`](#workspaceyml)
    - [projects](#projects)
    - [node](#node)
      - [version](#version)
      - [packageManager](#packagemanager)
      - [npm, pnpm, yarn](#npm-pnpm-yarn)
        - [version](#version-1)
      - [addEnginesConstraint](#addenginesconstraint)
      - [dedupeOnLockfileChange](#dedupeoninstall)
      - [syncProjectWorkspaceDependencies](#syncprojectworkspacedependencies)
      - [syncVersionManagerConfig](#syncversionmanagerconfig)
    - [typescript](#typescript)
      - [projectConfigFileName](#projectconfigfilename)
      - [rootConfigFileName](#rootconfigfilename)
      - [syncProjectReferences](#syncprojectreferences)
    - [vcs](#vcs)
      - [manager](#manager)
      - [defaultBranch](#defaultbranch)
  - [`project.yml`](#projectyml)
    - [fileGroups](#filegroups)
    - [tasks](#tasks)
      - [args](#args)
      - [deps](#deps)
      - [env](#env)
      - [inputs](#inputs)
      - [outputs](#outputs)
      - [options](#options)
      - [type](#type)

A workspace is a directory that contains [projects](./project.md), manages a
[toolchain](./toolchain.md), and is typically coupled with a VCS repository. The root of a workspace
is denoted by a `.moon` folder and a `package.json`.

By default Moon has been designed for monorepos, but can also be used for polyrepos.

## Configuration

Configurations that apply to the entire workspace are located within a `.moon` folder at the
workspace root.

> This folder _must_ be relative to the root `package.json` and it's associated lock file.

### `workspace.yml`

### `project.yml`

The `.moon/project.yml` file configures settings that are inherited by _every_ project in the
workspace. Projects can override these settings within their `<project path>/project.yml`
([view the projects docs for more information](./project.md#configuration)).
