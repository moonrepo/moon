#!/usr/bin/env pwsh
# Copyright 2022 moonrepo, Inc.

# Thanks to Deno for the original script:
# https://github.com/denoland/deno_install/blob/master/install.ps1

$ErrorActionPreference = 'Stop'

$Version = "latest"
$Target = "moon-x86_64-pc-windows-msvc.exe"

if ($Args.Length -eq 1) {
  $Version = $Args.Get(0)
}

$DownloadUrl = if ($Version -eq "latest") {
  "https://github.com/moonrepo/moon/releases/latest/download/${Target}"
} else {
  "https://github.com/moonrepo/moon/releases/download/v${Version}/${Target}"
}

$InstallDir = if ($env:MOON_INSTALL_DIR) {
  $env:MOON_INSTALL_DIR
} else {
  "${Home}\.moon\bin"
}

$BinPath = "${InstallDir}\moon.exe"

if (!(Test-Path $InstallDir)) {
  New-Item $InstallDir -ItemType Directory | Out-Null
}

curl.exe -Lo $BinPath $DownloadUrl

# Windows doesn't support a "shared binaries" type of folder,
# so instead of symlinking, we add the install dir to $PATH.
$User = [System.EnvironmentVariableTarget]::User
$Path = [System.Environment]::GetEnvironmentVariable('Path', $User)

if (!(";${Path};".ToLower() -like "*;${InstallDir};*".ToLower())) {
  [System.Environment]::SetEnvironmentVariable('Path', "${InstallDir};${Path}", $User)
  $Env:Path = "${InstallDir};${Env:Path}"
}

Write-Output "Successfully installed moon to ${BinPath}"
Write-Output "Run 'moon --help' to get started!"
Write-Output ""
Write-Output "Need help? Join our Discord https://discord.gg/qCh9MEynv2"

if ($env:MOON_DEBUG -eq "true") {
	Write-Output ""
	Write-Output "target=${Target}"
	Write-Output "download_url=${DownloadUrl}"
	Write-Output "bin_path=${BinPath}"
}
