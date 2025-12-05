#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

New-Item -Path ./file.txt -ItemType File -Force | Out-Null
New-Item -Path ./folder -ItemType Directory -Force | Out-Null
New-Item -Path ./folder/subfile.txt -ItemType File -Force | Out-Null
