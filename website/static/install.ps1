#!/usr/bin/env pwsh
# Copyright 2022 moonrepo LLC

# Thanks to Deno for the original script:
# https://github.com/denoland/deno_install/blob/master/install.ps1

$ErrorActionPreference = 'Stop'

$Target = "moon-x86_64-pc-windows-msvc.exe"
$Version = "latest"

if ($Args.Length -eq 1) {
  $Version = $Args.Get(0)
}

$DownloadUrl = if ($Version -eq "latest") {
  "https://github.com/moonrepo/moon/releases/latest/download/${Target}"
} else {
  "https://github.com/moonrepo/moon/releases/download/%40moonrepo%2Fcli%40/${Version}/${Target}"
}

$InstallDir = "${Home}\.moon\tools\moon\${Version}"
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
  [System.Environment]::SetEnvironmentVariable('Path', "${Path};${InstallDir}", $User)
  $Env:Path += ";${InstallDir}"
}

Write-Output "Successfully installed moon to ${BinPath}"
Write-Output "Run 'moon --help' to get started!"
Write-Output ""
Write-Output "Need help? Join our Discord https://discord.gg/qCh9MEynv2"

if ($env:MOON_TEST -eq "true") {
	Write-Output ""
	Write-Output "target=${Target}"
	Write-Output "download_url=${DownloadUrl}"
	Write-Output "bin_path=${BinPath}"
}
