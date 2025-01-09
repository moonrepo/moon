#!/usr/bin/env pwsh
# Copyright 2022 moonrepo, Inc.

# Thanks to Deno for the original script:
# https://github.com/denoland/deno_install/blob/master/install.ps1

$ErrorActionPreference = 'Stop'

$Target = "proto_cli-x86_64-pc-windows-msvc"

# Determine version and arguments

$Version = "latest"

$SetupArgs = New-Object -TypeName "System.Collections.ArrayList"
$SetupArgs.Add("setup") | Out-Null

ForEach ($Arg in $Args){
  if ($Arg.StartsWith("-")) {
    $SetupArgs.Add($Arg) | Out-Null
  } else {
    $Version = $Arg;
  }
}

$DownloadUrl = if ($Version -eq "latest") {
  "https://github.com/moonrepo/proto/releases/latest/download/${Target}.zip"
} else {
  "https://github.com/moonrepo/proto/releases/download/v${Version}/${Target}.zip"
}

$TempDir = "${HOME}\.proto\temp\proto\${Target}"
$DownloadFile = "${TempDir}.zip"

$InstallDir = if ($env:PROTO_HOME) {
  "$($env:PROTO_HOME)\bin"
} else {
  "${Home}\.proto\bin"
}

$BinPath = "${InstallDir}\proto.exe"
$ShimPath = "${InstallDir}\proto-shim.exe"

# Download and unpack in temp dir

if (!(Test-Path $TempDir)) {
  New-Item $TempDir -ItemType Directory | Out-Null
}

$wc = New-Object Net.Webclient
$wc.downloadFile($DownloadUrl, $DownloadFile)

if ($env:PROTO_DEBUG -eq "true") {
  Expand-Archive -Path $DownloadFile -DestinationPath $TempDir -PassThru
} else {
  Expand-Archive -Path $DownloadFile -DestinationPath $TempDir
}

# Move to bin dir and clean up

if (!(Test-Path $InstallDir)) {
  New-Item $InstallDir -ItemType Directory | Out-Null
}

Copy-Item "${TempDir}\proto.exe" -Destination $BinPath -Force

if (Test-Path "${TempDir}\proto-shim.exe") {
  Copy-Item "${TempDir}\proto-shim.exe" -Destination $ShimPath -Force
}

Remove-Item $TempDir -Recurse -Force
Remove-Item $DownloadFile -Force

if ($env:PROTO_DEBUG -eq "true") {
  Write-Output "target=${Target}"
	Write-Output "download_url=${DownloadUrl}"
	Write-Output "bin_path=${BinPath}"
	Write-Output "shim_path=${ShimPath}"
	Write-Output ""
}

# Run setup script to update shells

if (-not $env:PROTO_LOG) {
  $env:PROTO_LOG = "error"
}

$env:STARBASE_FORCE_TTY = "true"

# We can't automatically run setup for the following reasons:
#   - Interactive prompts don't work in piped commands
#   - There's no way to pass arguments to `iex`

# Start-Process -FilePath $BinPath -ArgumentList $SetupArgs -NoNewWindow -Wait

Write-Output "Successfully installed proto! Run the command below to finish setting"
Write-Output "up your environment, by modifying PATH, and configuring your shell."
Write-Output ""
Write-Output "  $BinPath setup"
Write-Output ""
Write-Output "Pass --help to view all available options."
Write-Output "Need help? Join our Discord https://discord.gg/qCh9MEynv2"

