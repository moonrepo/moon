#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

Write-Output "Args: ${args}"

for ($i = 0; $i -lt $args.Count; $i++) {
    $index = $i + 1
    $val = $args[$i]
    Write-Output "Arg ${index}: ${val} ('${val}')"
}
