#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

Get-ChildItem Env: | Where-Object { $_.Name -like 'MOON_*' } | ForEach-Object {
    $name = $_.Name
    Write-Output "$name=$($_.Value)"
}
