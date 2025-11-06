#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

Write-Output "stdout"
# Write-Error "stderr"
[Console]::Error.WriteLine('stderr')
