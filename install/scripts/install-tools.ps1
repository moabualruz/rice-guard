# rice-guard — Install all scanner and fixer tool dependencies (Windows).
#
# Usage:
#   .\install-tools.ps1              # Install everything
#   .\install-tools.ps1 -Scanners   # Scanners only
#   .\install-tools.ps1 -Fixers     # Fixers only
#   .\install-tools.ps1 -Lang go    # Fixers for a specific language
#   .\install-tools.ps1 -CheckOnly  # Check what's installed (dry run)
#
# Supported package managers: scoop, winget, go, pip, npm, gem, cargo, composer
# Platform: Windows (PowerShell 5.1+ / pwsh 7+)

param(
    [switch]$Scanners,
    [switch]$Fixers,
    [string]$Lang = "",
    [switch]$CheckOnly,
    [switch]$Help
)

$ErrorActionPreference = "Continue"

# ── Helpers ──────────────────────────────────────────────────────────────────
function Write-Info    { param([string]$Msg) Write-Host "[INFO] $Msg" -ForegroundColor Blue }
function Write-Ok      { param([string]$Msg) Write-Host "  [OK] $Msg" -ForegroundColor Green }
function Write-Skip    { param([string]$Msg) Write-Host "[SKIP] $Msg" -ForegroundColor Yellow }
function Write-Fail    { param([string]$Msg) Write-Host "[FAIL] $Msg" -ForegroundColor Red }
function Write-Warn    { param([string]$Msg) Write-Host "[WARN] $Msg" -ForegroundColor DarkYellow }
function Write-Section { param([string]$Msg) Write-Host "`n=== $Msg ===" -ForegroundColor Cyan }

# ── State ────────────────────────────────────────────────────────────────────
$script:Installed  = 0
$script:Skipped    = 0
$script:Failed     = 0
$script:FailedList = @()
$script:ToolsDir   = Join-Path $env:USERPROFILE ".rice-guard\tools"

if ($Help) {
    Write-Host "Usage: install-tools.ps1 [-Scanners] [-Fixers] [-Lang LANG] [-CheckOnly]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Scanners   Install scanners only"
    Write-Host "  -Fixers     Install fixers only"
    Write-Host "  -Lang       Install fixers for a specific language (go, python, js, rust, etc.)"
    Write-Host "  -CheckOnly  Only check what's installed; don't install anything"
    exit 0
}

$Mode = "all"
if ($Scanners) { $Mode = "scanners" }
if ($Fixers)   { $Mode = "fixers" }

function Test-Command {
    param([string]$Name)
    $null = Get-Command $Name -ErrorAction SilentlyContinue
    return $?
}

function Try-Install {
    param(
        [string]$Name,
        [string]$CheckCmd,
        [string]$InstallCmd,
        [string]$PkgMgr = ""
    )

    # Check if already installed
    try {
        $result = Invoke-Expression $CheckCmd 2>&1
        if ($LASTEXITCODE -eq $null -or $LASTEXITCODE -eq 0) {
            Write-Skip "$Name (already installed)"
            $script:Skipped++
            return
        }
    } catch {
        # Not installed — proceed
    }

    if ($CheckOnly) {
        Write-Fail "$Name (not installed)"
        $script:Failed++
        $script:FailedList += $Name
        return
    }

    # Check if package manager exists
    if ($PkgMgr -and -not (Test-Command $PkgMgr)) {
        Write-Fail "$Name (requires $PkgMgr - not found)"
        $script:Failed++
        $script:FailedList += $Name
        return
    }

    Write-Info "Installing $Name..."
    try {
        Invoke-Expression $InstallCmd 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0 -or $LASTEXITCODE -eq $null) {
            Write-Ok "$Name installed"
            $script:Installed++
        } else {
            Write-Fail "$Name - install command returned exit code $LASTEXITCODE"
            $script:Failed++
            $script:FailedList += $Name
        }
    } catch {
        Write-Fail "$Name - $($_.Exception.Message)"
        $script:Failed++
        $script:FailedList += $Name
    }
}

# Download a JAR tool and create .cmd + bash wrapper in scoop shims dir.
function Install-JarTool {
    param(
        [string]$Name,
        [string]$DownloadUrl,
        [string]$CheckCmd
    )

    # Check if already installed
    try {
        $result = Invoke-Expression $CheckCmd 2>&1
        if ($LASTEXITCODE -eq $null -or $LASTEXITCODE -eq 0) {
            Write-Skip "$Name (already installed)"
            $script:Skipped++
            return
        }
    } catch {
        # Not installed
    }

    if ($CheckOnly) {
        Write-Fail "$Name (not installed)"
        $script:Failed++
        $script:FailedList += $Name
        return
    }

    if (-not (Test-Command "java")) {
        Write-Fail "$Name (requires java - not found)"
        $script:Failed++
        $script:FailedList += $Name
        return
    }

    # Create tools directory
    if (-not (Test-Path $script:ToolsDir)) {
        New-Item -ItemType Directory -Force -Path $script:ToolsDir | Out-Null
    }

    $jarPath = Join-Path $script:ToolsDir "$Name.jar"
    Write-Info "Downloading $Name JAR..."

    try {
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $jarPath -UseBasicParsing
    } catch {
        Write-Fail "$Name - download failed: $($_.Exception.Message)"
        $script:Failed++
        $script:FailedList += $Name
        return
    }

    # Create .cmd wrapper in scoop shims (already in PATH)
    $shimsDir = Join-Path $env:USERPROFILE "scoop\shims"
    if (-not (Test-Path $shimsDir)) {
        # Fallback: create in tools dir and warn
        $shimsDir = $script:ToolsDir
        Write-Warn "scoop shims dir not found — placing wrappers in $shimsDir (add to PATH)"
    }

    $cmdContent = "@echo off`r`njava -jar `"%USERPROFILE%\.rice-guard\tools\$Name.jar`" %*"
    Set-Content -Path (Join-Path $shimsDir "$Name.cmd") -Value $cmdContent

    $shContent = "#!/bin/sh" + "`n" + "java -jar " + '"$HOME/.rice-guard/tools/' + $Name + '.jar" "$@"'
    Set-Content -Path (Join-Path $shimsDir $Name) -Value $shContent -NoNewline

    Write-Ok "$Name installed (JAR + wrapper)"
    $script:Installed++
}

# ── Package manager detection ───────────────────────────────────────────────
$HasScoop  = Test-Command "scoop"
$HasWinget = Test-Command "winget"
$HasGo     = Test-Command "go"
$HasPip    = Test-Command "pip"
$HasNpm    = Test-Command "npm"
$HasGem    = Test-Command "gem"
$HasCargo  = Test-Command "cargo"
$HasJava   = Test-Command "java"

Write-Section "Package Manager Status"
$managers = @(
    @("scoop",  $HasScoop),
    @("winget", $HasWinget),
    @("go",     $HasGo),
    @("pip",    $HasPip),
    @("npm",    $HasNpm),
    @("gem",    $HasGem),
    @("cargo",  $HasCargo),
    @("java",   $HasJava)
)
foreach ($m in $managers) {
    $status = if ($m[1]) { "found" } else { "not found" }
    $color  = if ($m[1]) { "Green" } else { "Red" }
    Write-Host "  $($m[0]): " -NoNewline
    Write-Host $status -ForegroundColor $color
}

# ── JAVA_HOME validation ────────────────────────────────────────────────────
if ($HasJava -and $env:JAVA_HOME) {
    if (-not (Test-Path $env:JAVA_HOME)) {
        Write-Warn "JAVA_HOME points to non-existent directory: $env:JAVA_HOME"
        # Try to auto-fix by finding the actual JDK
        $javaPath = (Get-Command java).Source
        $jdkHome = (Split-Path (Split-Path $javaPath))
        if (Test-Path $jdkHome) {
            Write-Info "Auto-fixing JAVA_HOME to: $jdkHome"
            $env:JAVA_HOME = $jdkHome
            [Environment]::SetEnvironmentVariable("JAVA_HOME", $jdkHome, "User")
            Write-Ok "JAVA_HOME updated permanently"
        }
    }
}

# ── Composer global bin PATH ────────────────────────────────────────────────
$ComposerBin = ""
if (Test-Command "composer") {
    $ComposerBin = Join-Path $env:APPDATA "Composer\vendor\bin"
    if (Test-Path $ComposerBin) {
        # Add to current session PATH if not present
        if ($env:PATH -notlike "*$ComposerBin*") {
            $env:PATH = "$env:PATH;$ComposerBin"
        }
        # Add to user PATH permanently if not present
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($userPath -notlike "*$ComposerBin*") {
            [Environment]::SetEnvironmentVariable("Path", "$userPath;$ComposerBin", "User")
            Write-Info "Added composer global bin to user PATH: $ComposerBin"
        }
    }
}

# ── .NET global tools PATH ─────────────────────────────────────────────────
$DotnetToolsDir = Join-Path $env:USERPROFILE ".dotnet\tools"
if (Test-Path $DotnetToolsDir) {
    if ($env:PATH -notlike "*$DotnetToolsDir*") {
        $env:PATH = "$env:PATH;$DotnetToolsDir"
    }
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$DotnetToolsDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$DotnetToolsDir", "User")
        Write-Info "Added .NET global tools to user PATH: $DotnetToolsDir"
    }
}

# ═══════════════════════════════════════════════════════════════════════════════
# SCANNERS
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Scanners {
    Write-Section "Scanners"

    # semgrep
    if ($HasPip) {
        Try-Install "semgrep" "semgrep --version" "pip install semgrep" "pip"
    } else {
        Write-Fail "semgrep (needs pip)"
        $script:Failed++; $script:FailedList += "semgrep"
    }

    # trivy
    if ($HasScoop) {
        Try-Install "trivy" "trivy --version" "scoop install trivy" "scoop"
    } elseif ($HasWinget) {
        Try-Install "trivy" "trivy --version" "winget install --id AquaSecurity.Trivy -e --accept-source-agreements --accept-package-agreements" "winget"
    } else {
        Write-Fail "trivy (needs scoop or winget)"
        $script:Failed++; $script:FailedList += "trivy"
    }

    # gitleaks
    if ($HasScoop) {
        Try-Install "gitleaks" "gitleaks version" "scoop install gitleaks" "scoop"
    } elseif ($HasWinget) {
        Try-Install "gitleaks" "gitleaks version" "winget install --id Gitleaks.Gitleaks -e --accept-source-agreements --accept-package-agreements" "winget"
    } else {
        Write-Fail "gitleaks (needs scoop or winget)"
        $script:Failed++; $script:FailedList += "gitleaks"
    }

    # jscpd
    Try-Install "jscpd" "jscpd --version" "npm install -g jscpd" "npm"

    # scc
    if ($HasScoop) {
        Try-Install "scc" "scc --version" "scoop install scc" "scoop"
    } elseif ($HasGo) {
        Try-Install "scc" "scc --version" "go install github.com/boyter/scc/v3@latest" "go"
    } else {
        Write-Fail "scc (needs scoop or go)"
        $script:Failed++; $script:FailedList += "scc"
    }
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Go
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-Go {
    Write-Section "Fixers - Go"
    Try-Install "gofumpt"       "gofumpt --version"       "go install mvdan.cc/gofumpt@latest"                                    "go"
    Try-Install "goimports"     "Get-Command goimports"    "go install golang.org/x/tools/cmd/goimports@latest"                    "go"
    Try-Install "golangci-lint" "golangci-lint --version"  "go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest"  "go"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Python
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-Python {
    Write-Section "Fixers - Python"
    Try-Install "ruff"      "ruff --version"      "pip install ruff"      "pip"
    Try-Install "pip-audit" "pip-audit --version" "pip install pip-audit" "pip"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — JavaScript / TypeScript
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-JS {
    Write-Section "Fixers - JavaScript / TypeScript"
    Try-Install "biome"    "biome --version"    "npm install -g @biomejs/biome" "npm"
    Try-Install "prettier" "prettier --version" "npm install -g prettier"       "npm"
    Try-Install "eslint"   "eslint --version"   "npm install -g eslint"         "npm"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Rust
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-Rust {
    Write-Section "Fixers - Rust"
    Try-Install "rustfmt"        "rustfmt --version"        "rustup component add rustfmt"    "rustup"
    Try-Install "clippy"         "cargo clippy --version"   "rustup component add clippy"     "rustup"
    Try-Install "cargo-outdated" "cargo outdated --version" "cargo install cargo-outdated"    "cargo"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Ruby
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-Ruby {
    Write-Section "Fixers - Ruby"
    Try-Install "rubocop"       "rubocop --version"    "gem install rubocop"       "gem"
    Try-Install "bundler-audit" "bundle-audit version" "gem install bundler-audit" "gem"
    Try-Install "brakeman"      "brakeman --version"   "gem install brakeman"      "gem"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Java
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-Java {
    Write-Section "Fixers - Java"

    # google-java-format: try scoop, fall back to JAR
    if ($HasScoop) {
        Try-Install "google-java-format" "google-java-format --version" "scoop install extras/google-java-format" "scoop"
    } else {
        Install-JarTool "google-java-format" `
            "https://github.com/google/google-java-format/releases/latest/download/google-java-format-all-deps.jar" `
            "google-java-format --version"
    }

    # checkstyle: no reliable scoop manifest — use JAR download
    Install-JarTool "checkstyle" `
        "https://github.com/checkstyle/checkstyle/releases/download/checkstyle-13.3.0/checkstyle-13.3.0-all.jar" `
        "checkstyle --version"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Kotlin
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-Kotlin {
    Write-Section "Fixers - Kotlin"

    # detekt: scoop has it
    if ($HasScoop) {
        Try-Install "detekt" "detekt --version" "scoop install main/detekt" "scoop"
    } else {
        Write-Info "detekt: install via scoop (scoop install detekt) or download from https://github.com/detekt/detekt/releases"
        $script:Failed++; $script:FailedList += "detekt"
    }

    # ktfmt: no scoop manifest — JAR download
    Install-JarTool "ktfmt" `
        "https://github.com/facebook/ktfmt/releases/download/v0.61/ktfmt-0.61-with-dependencies.jar" `
        "ktfmt --version"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — PHP
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-PHP {
    Write-Section "Fixers - PHP"

    if (-not (Test-Command "php")) {
        Write-Info "PHP not found. Install via: scoop install php"
        $script:Failed += 2; $script:FailedList += "php-cs-fixer"; $script:FailedList += "phpstan"
        return
    }

    # Check and enable openssl if needed
    $hasOpenssl = php -m 2>&1 | Select-String "openssl"
    if (-not $hasOpenssl) {
        Write-Warn "PHP openssl extension not loaded — attempting to enable..."
        $phpDir = Split-Path (Get-Command php).Source
        $phpIni = Join-Path $phpDir "php.ini"
        if (-not (Test-Path $phpIni)) {
            $phpIniProd = Join-Path $phpDir "php.ini-production"
            if (Test-Path $phpIniProd) {
                Copy-Item $phpIniProd $phpIni
                Write-Info "Created php.ini from production template"
            }
        }
        if (Test-Path $phpIni) {
            (Get-Content $phpIni) -replace '^;extension=openssl', 'extension=openssl' | Set-Content $phpIni
            $hasOpenssl = php -m 2>&1 | Select-String "openssl"
            if ($hasOpenssl) {
                Write-Ok "openssl extension enabled"
            } else {
                Write-Fail "Could not enable openssl — php_openssl.dll may be missing"
                Write-Info "Reinstall PHP via scoop: scoop install php"
                $script:Failed += 2; $script:FailedList += "php-cs-fixer"; $script:FailedList += "phpstan"
                return
            }
        }
    }

    if (-not (Test-Command "composer")) {
        if ($HasScoop) {
            Write-Info "Installing composer via scoop..."
            scoop install composer 2>&1 | Out-Null
        } else {
            Write-Fail "composer not found (install via: scoop install composer)"
            $script:Failed += 2; $script:FailedList += "php-cs-fixer"; $script:FailedList += "phpstan"
            return
        }
    }

    Try-Install "php-cs-fixer" "php-cs-fixer --version" "composer global require friendsofphp/php-cs-fixer" "composer"
    Try-Install "phpstan"      "phpstan --version"      "composer global require phpstan/phpstan"             "composer"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Shell
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-Shell {
    Write-Section "Fixers - Shell"
    if ($HasGo) {
        Try-Install "shfmt" "shfmt --version" "go install mvdan.cc/sh/v3/cmd/shfmt@latest" "go"
    } elseif ($HasScoop) {
        Try-Install "shfmt" "shfmt --version" "scoop install shfmt" "scoop"
    } else {
        Write-Fail "shfmt (needs go or scoop)"
        $script:Failed++; $script:FailedList += "shfmt"
    }

    if ($HasScoop) {
        Try-Install "shellcheck" "shellcheck --version" "scoop install shellcheck" "scoop"
    } else {
        Write-Info "shellcheck: install via scoop (scoop install shellcheck)"
        $script:Failed++; $script:FailedList += "shellcheck"
    }
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Dart / Flutter
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-Dart {
    Write-Section "Fixers - Dart / Flutter"
    if (Test-Command "dart") {
        Write-Ok "dart SDK found (includes dart format, dart fix, dart analyze)"
    } else {
        Write-Info "Install the Dart SDK: https://dart.dev/get-dart"
        $script:Failed++; $script:FailedList += "dart-sdk"
    }
    if (Test-Command "flutter") {
        Write-Ok "flutter SDK found (includes flutter analyze)"
    } else {
        Write-Info "Install Flutter SDK: https://flutter.dev/docs/get-started/install"
        $script:Failed++; $script:FailedList += "flutter-sdk"
    }
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — C# / .NET
# ═══════════════════════════════════════════════════════════════════════════════
function Install-Fixers-CSharp {
    Write-Section "Fixers - C# / .NET"
    if (Test-Command "dotnet") {
        # Check if SDK is installed (not just runtime)
        $sdks = dotnet --list-sdks 2>&1
        if ($sdks -match "No SDKs were found" -or [string]::IsNullOrWhiteSpace($sdks)) {
            Write-Warn "dotnet runtime found but no SDK installed"
            Write-Info "Install .NET SDK for dotnet format + dotnet-outdated: https://dotnet.microsoft.com/download"
            Write-Info "dotnet format and dotnet-outdated require the SDK, not just the runtime"
            $script:Failed++; $script:FailedList += "dotnet-sdk"
        } else {
            Write-Ok "dotnet SDK found (includes dotnet format)"
            Try-Install "dotnet-outdated" "dotnet outdated --version" "dotnet tool install -g dotnet-outdated-tool" "dotnet"
        }
    } else {
        Write-Info "Install .NET SDK: https://dotnet.microsoft.com/download"
        $script:Failed++; $script:FailedList += "dotnet-sdk"
    }
}

# ═══════════════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════════════

Write-Host "rice-guard - Tool Dependency Installer" -ForegroundColor Cyan
Write-Host "Mode: $Mode  Lang: $(if ($Lang) { $Lang } else { 'all' })  Check-only: $CheckOnly" -ForegroundColor Yellow

if ($Mode -eq "all" -or $Mode -eq "scanners") {
    Install-Scanners
}

if ($Mode -eq "all" -or $Mode -eq "fixers") {
    $langMatch = $Lang.ToLower()
    if (-not $langMatch -or $langMatch -eq "go")                                                  { Install-Fixers-Go }
    if (-not $langMatch -or $langMatch -eq "python")                                              { Install-Fixers-Python }
    if (-not $langMatch -or $langMatch -in @("javascript","typescript","js","ts"))                 { Install-Fixers-JS }
    if (-not $langMatch -or $langMatch -eq "rust")                                                { Install-Fixers-Rust }
    if (-not $langMatch -or $langMatch -eq "ruby")                                                { Install-Fixers-Ruby }
    if (-not $langMatch -or $langMatch -eq "java")                                                { Install-Fixers-Java }
    if (-not $langMatch -or $langMatch -eq "kotlin")                                              { Install-Fixers-Kotlin }
    if (-not $langMatch -or $langMatch -eq "php")                                                 { Install-Fixers-PHP }
    if (-not $langMatch -or $langMatch -in @("shell","bash"))                                     { Install-Fixers-Shell }
    if (-not $langMatch -or $langMatch -in @("dart","flutter"))                                   { Install-Fixers-Dart }
    if (-not $langMatch -or $langMatch -in @("csharp","dotnet"))                                  { Install-Fixers-CSharp }
}

# ── Summary ──────────────────────────────────────────────────────────────────
Write-Section "Summary"
Write-Host "  Installed: $($script:Installed)" -ForegroundColor Green
Write-Host "  Skipped:   $($script:Skipped) (already installed)" -ForegroundColor Yellow
Write-Host "  Failed:    $($script:Failed)" -ForegroundColor Red

if ($script:FailedList.Count -gt 0) {
    Write-Host "`nFailed tools:" -ForegroundColor Red
    foreach ($tool in $script:FailedList) {
        Write-Host "  - $tool" -ForegroundColor Red
    }
}

if ($script:Failed -gt 0) {
    exit 1
}
