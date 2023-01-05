#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

if (Test-Path env:PROTO_DEBUG) \{
    $DebugPreference = 'Continue'
}

[Environment]::SetEnvironmentVariable('PROTO_ROOT', '{root}', 'Process')

{{ if install_dir }}
[Environment]::SetEnvironmentVariable('PROTO_{constant_name}_DIR', '{install_dir}', 'Process')
{{ endif }}

{{ if version }}
[Environment]::SetEnvironmentVariable('PROTO_{constant_name}_VERSION', '{version}', 'Process')
{{ endif }}

{{ if parent_bin }}
if (Test-Path env:PROTO_{parent_bin}_BIN) \{
    $parent = $Env:PROTO_{parent_bin}_BIN
} else \{
    $parent = "{parent_bin}.exe"
}

& "$parent" "{bin_path}" @Args
{{ else }}

& "{bin_path}" @Args
{{ endif }}

exit $LASTEXITCODE
