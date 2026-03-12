# rice-guard installer for Windows — downloads the x86_64-pc-windows-msvc binary.
$ErrorActionPreference = "Stop"

$Repo = "user/rice-guard"  # TODO: update to real GitHub owner
$Version = if ($env:RICE_GUARD_VERSION) { $env:RICE_GUARD_VERSION } else { "latest" }
$InstallDir = if ($env:RICE_GUARD_INSTALL_DIR) { $env:RICE_GUARD_INSTALL_DIR } else { "$env:LOCALAPPDATA\rice-guard" }
$Target = "x86_64-pc-windows-msvc"

if ($Version -eq "latest") {
    $Url = "https://github.com/$Repo/releases/latest/download/rice-guard-$Target.zip"
} else {
    $Url = "https://github.com/$Repo/releases/download/v$Version/rice-guard-$Target.zip"
}

Write-Host "Installing rice-guard for $Target..."
Write-Host "  From: $Url"
Write-Host "  To:   $InstallDir\rice-guard.exe"

$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "rice-guard-install"
New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null

try {
    $ZipPath = Join-Path $TmpDir "rice-guard.zip"
    Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing

    Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item (Join-Path $TmpDir "rice-guard.exe") (Join-Path $InstallDir "rice-guard.exe") -Force

    # Add to user PATH if not present
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
        Write-Host "Added $InstallDir to user PATH."
    }

    Write-Host ""
    Write-Host "Installed rice-guard to $InstallDir\rice-guard.exe"
} finally {
    Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
}
