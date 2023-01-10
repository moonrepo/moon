@ECHO OFF

SETLOCAL
SET "PROTO_ROOT={root}"

{{ if install_dir }}
SET "PROTO_{name | uppercase}_DIR={install_dir}"
{{ endif }}

{{ if version }}
SET "PROTO_{name | uppercase}_VERSION={version}"
{{ endif }}

{{ if parent_name }}
SET "parent=%PROTO_{parent_name | uppercase}_BIN%"
IF "%parent%" == "" SET "parent={parent_name}"

"%parent%" "{bin_path}" %*
{{ else }}

"{bin_path}" %*
{{ endif }}
