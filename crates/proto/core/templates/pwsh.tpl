#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

if (Test-Path env:PROTO_DEBUG) \{
    $DebugPreference = 'Continue'
}

[Environment]::SetEnvironmentVariable('PROTO_ROOT', '{root}', 'Process')

{{ if install_dir }}
[Environment]::SetEnvironmentVariable('PROTO_{name | uppercase}_DIR', '{install_dir}', 'Process')
{{ endif }}

{{ if version }}
[Environment]::SetEnvironmentVariable('PROTO_{name | uppercase}_VERSION', '{version}', 'Process')
{{ endif }}

{{ if parent_name }}
if (Test-Path env:PROTO_{parent_name | uppercase}_BIN) \{
    $parent = $Env:PROTO_{parent_name | uppercase}_BIN
} else \{
    $parent = "{parent_name}.exe"
}

& "$parent" "{bin_path}" @Args
{{ else }}

& "{bin_path}" @Args
{{ endif }}

exit $LASTEXITCODE
