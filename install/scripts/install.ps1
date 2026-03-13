# rguard installer for Windows — downloads the x86_64-pc-windows-msvc binary.
$ErrorActionPreference = "Stop"

$Repo = "moabualruz/rice-guard"  # TODO: update to real GitHub owner
$Version = if ($env:RGUARD_VERSION) { $env:RGUARD_VERSION } else { "latest" }
$InstallDir = if ($env:RGUARD_INSTALL_DIR) { $env:RGUARD_INSTALL_DIR } else { "$env:LOCALAPPDATA\rguard" }
$Target = "x86_64-pc-windows-msvc"

if ($Version -eq "latest") {
    $Url = "https://github.com/$Repo/releases/latest/download/rguard-$Target.zip"
} else {
    $Url = "https://github.com/$Repo/releases/download/v$Version/rguard-$Target.zip"
}

Write-Host "Installing rguard for $Target..."
Write-Host "  From: $Url"
Write-Host "  To:   $InstallDir\rguard.exe"

$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "rguard-install"
New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null

try {
    $ZipPath = Join-Path $TmpDir "rguard.zip"
    Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing

    Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item (Join-Path $TmpDir "rguard.exe") (Join-Path $InstallDir "rguard.exe") -Force

    # Add to user PATH if not present
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
        Write-Host "Added $InstallDir to user PATH."
    }

    Write-Host ""
    Write-Host "Installed rguard to $InstallDir\rguard.exe"
} finally {
    Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
}
