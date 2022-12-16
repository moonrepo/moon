---
title: Terminology
---

| Term                          | Description                                                                                                                                             |
| :---------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Action                        | A node within the dependency graph that gets executed by the action runner.                                                                             |
| Action runner                 | Executes actions from our dependency graph in topological order.                                                                                        |
| Affected                      | Touched by an explicit set of inputs or sources.                                                                                                        |
| Cache                         | Files and outputs that are stored on the file system to provide incremental builds and increased performance.                                           |
| CI                            | Continuous integration. An environment where tests, builds, lints, etc, are continuously ran on every pull/merge request.                               |
| Dependency graph              | A directed acyclic graph (DAG) of targets to run and their dependencies.                                                                                |
| Downstream                    | Dependents or consumers of the item in question.                                                                                                        |
| [Generator](./guides/codegen) | Generates code from pre-defined templates.                                                                                                              |
| Hash                          | A unique SHA256 identifier that represents the result of a ran task.                                                                                    |
| Hashing                       | The mechanism of generating a hash based on multiple sources: inputs, dependencies, configs, etc.                                                       |
| LTS                           | Long-term support.                                                                                                                                      |
| Dependency manager            | Installs and manages dependencies for a specific tool (`npm`), using a manifest file (`package.json`).                                                  |
| Platform                      | An internal concept representing the integration of a programming language (tool) within moon, and also the environment + language that a task runs in. |
| Primary target                | The target that was explicitly ran, and is the dependee of transitive targets.                                                                          |
| [Project][project]            | An collection of source and test files, configurations, a manifest and dependencies, and much more. Exists within a [workspace][workspace]              |
| Revision                      | In the context of a VCS: a branch, revision, commit, hash, or point in history.                                                                         |
| Runtime                       | An internal concept representing the platform + version of a tool.                                                                                      |
| [Target][target]              | A label and reference to a task within the project, in the format of `project:task`.                                                                    |
| [Task][task]                  | A command to run within the context of and configured in a [project][project].                                                                          |
| Template                      | A collection of files that get scaffolded by a generator.                                                                                               |
| Template file                 | An individual file within a template.                                                                                                                   |
| Template variable             | A value that is interpolated within a template file and its file system path.                                                                           |
| [Token][token]                | A value within task configuration that is substituted at runtime.                                                                                       |
| Tool                          | A programming language or dependency manager within the [toolchain][toolchain].                                                                         |
| [Toolchain][toolchain]        | Installs and manages tools within the [workspace][workspace].                                                                                           |
| Transitive target             | A target that is the dependency of the primary target, and must be ran before the primary.                                                              |
| Touched                       | A file that has been created, modified, deleted, or changed in any way.                                                                                 |
| Upstream                      | Dependencies or producers of the item in question.                                                                                                      |
| VCS                           | Version control system (like git or svn).                                                                                                               |
| [Workspace][workspace]        | Root of the moon installation, and houses one or many [projects][project]. _Also refers to package manager workspaces (like Yarn)._                     |

[project]: ./concepts/project
[target]: ./concepts/target
[task]: ./concepts/task
[token]: ./concepts/token
[toolchain]: ./concepts/toolchain
[workspace]: ./concepts/workspace
