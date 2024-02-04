#!/usr/bin/env pwsh
# Copyright 2022 moonrepo, Inc.

# Thanks to Deno for the original script:
# https://github.com/denoland/deno_install/blob/master/install.ps1

$ErrorActionPreference = 'Stop'

$Version = "latest"
$Target = "proto_cli-x86_64-pc-windows-msvc"

if ($Args.Length -eq 1) {
  $Version = $Args.Get(0)
}

$DownloadUrl = if ($Version -eq "latest") {
  "https://github.com/moonrepo/proto/releases/latest/download/${Target}.zip"
} else {
  "https://github.com/moonrepo/proto/releases/download/v${Version}/${Target}.zip"
}

$TempDir = "${HOME}\.proto\temp\proto\${Target}"
$DownloadFile = "${TempDir}.zip"

$InstallDir = if ($env:PROTO_INSTALL_DIR) {
  $env:PROTO_INSTALL_DIR
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

# Run setup script to update shells

$env:PROTO_LOG = "error"

$SetupArgs = New-Object System.Collections.Generic.List[System.Object]
$SetupArgs.Add('setup')

ForEach ($Arg in $Args){
    if ($Arg -eq "--no-profile" || $Arg -eq "--yes" || $Arg -eq "-y") {
        $SetupArgs.Add($Arg)
    }
}

$BinPath $SetupArgs

if ($env:PROTO_DEBUG -eq "true") {
	Write-Output ""
	Write-Output "target=${Target}"
	Write-Output "download_url=${DownloadUrl}"
	Write-Output "bin_path=${BinPath}"
	Write-Output "shim_path=${ShimPath}"
}
