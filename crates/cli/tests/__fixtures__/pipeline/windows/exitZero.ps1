#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

Write-Output "stdout"
Write-Error "stderr"

exit 0

Write-Output "This should not appear!"
