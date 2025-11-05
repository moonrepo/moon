#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

Write-Output "stdout"
Write-Error "stderr"

exit 1

Write-Output "This should not appear!"
