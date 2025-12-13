#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

Write-Output "Args: $($args -join ' ')"
Write-Output "Env: $env:MOON_AFFECTED_FILES"
