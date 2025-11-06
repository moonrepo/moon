#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

Write-Output "TEST_FOO=$env:TEST_FOO"
Write-Output "TEST_BAR=$env:TEST_BAR"
Write-Output "TEST_BAZ=$env:TEST_BAZ"
