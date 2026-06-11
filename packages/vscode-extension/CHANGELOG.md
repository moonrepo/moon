## 0.16.6

- Fixed our release scripts.

## 0.16.5

- Re-published previous version.

## 0.16.4

- Fixed `.config/moon` handling.

## 0.16.3

- Updated to use v2 JSON schemas. If you are using v1, you may see validation errors.

## 0.16.2

- Fixed more moon v2 issues.

## 0.16.1

- Fixed some moon v2 issues.

## 0.16.0

- Updated to support moon v2.

## 0.15.2

- Requires VS Code v1.103.0.
- Published to Open VSX.

## 0.15.1

- Added support for project `layer` in moon v1.39.

## 0.15.0

- Added MCP support. Requires VS Code v1.102.0.

## 0.14.0

- Added support for the task graph. Requires moon v1.30.
- Added support for `.pkl` config files.

## 0.13.0

- Added a new command that will generate local `yaml.schemas` settings.

## 0.12.0

- Requires VS Code ^1.77.0.
- Added support for internal tasks. They will not be displayed in the projects/tags view.

## 0.11.0

- Added `stack` support to the projects view. Will now categorize based on stack + type.

## 0.10.0

- Added YAML file validation for all moon configuration files.
  - Requires the `redhat.vscode-yaml` extension to be installed. VSCode should prompt you to install
    it.

## 0.9.0

- Added a "Tags" view for projects in the moon console sidebar.

## 0.8.0

- Added a `moon.logLevel` setting, to control the log level of all moon executed commands.
- Added support for multiple VS Code workspace folders.
  - When you open a file in another workspace, the moon panels will refresh.
- Removed support for moon < 1.0.
- Replaced `moon.workspaceRoot` setting with `moon.rootPrefixes`.

## 0.7.0

- Requires VS Code ^1.75.0.
- Added action graph support (requires moon >= 1.15).
- Added support for the `automation` project type.
- Added a `moon.hideTasks` setting, to hide tasks in the projects view.

## 0.6.0

- Added 19 new language icons (requires moon >= 0.25).

## 0.5.0

- Added dependency and project graph in-editor visualization support.

## 0.4.0

- Added support for 5 new language icons: Go, PHP, Python, Ruby, Rust

## 0.3.0

- Changes to `moon.yml` will now automatically refresh projects.
- Added file and folder icons to the `assets` folder.
  - This _does not_ associate them. You'll need to do that manually in your editor settings.

## 0.2.0

- Added `moon check` support to the project rows in the Projects view.

## 0.1.0

- Initial release!
