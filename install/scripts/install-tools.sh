#!/usr/bin/env bash
# rice-guard — Install all scanner and fixer tool dependencies.
#
# Usage:
#   ./install-tools.sh              # Install everything
#   ./install-tools.sh --scanners   # Scanners only
#   ./install-tools.sh --fixers     # Fixers only
#   ./install-tools.sh --lang go    # Fixers for a specific language
#   ./install-tools.sh --check      # Check what's installed (dry run)
#
# Supported package managers: brew, go, pip, npm, gem, cargo, composer, curl
# Platform: macOS / Linux
set -euo pipefail

# ── Colors ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# ── State ────────────────────────────────────────────────────────────────────
INSTALLED=0
SKIPPED=0
FAILED=0
FAILED_LIST=()
MODE="all"       # all | scanners | fixers
LANG_FILTER=""   # empty = all languages
CHECK_ONLY=false
TOOLS_DIR="$HOME/.rice-guard/tools"

# ── Argument parsing ─────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --scanners)  MODE="scanners"; shift ;;
        --fixers)    MODE="fixers"; shift ;;
        --lang)      LANG_FILTER="$2"; shift 2 ;;
        --check)     CHECK_ONLY=true; shift ;;
        -h|--help)
            echo "Usage: install-tools.sh [--scanners|--fixers] [--lang LANG] [--check]"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# ── Helpers ──────────────────────────────────────────────────────────────────
info()    { echo -e "${BLUE}[INFO]${NC} $*"; }
ok()      { echo -e "${GREEN}  [OK]${NC} $*"; }
skip()    { echo -e "${YELLOW}[SKIP]${NC} $*"; }
fail()    { echo -e "${RED}[FAIL]${NC} $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; }
section() { echo -e "\n${CYAN}═══ $* ═══${NC}"; }

has() { command -v "$1" &>/dev/null; }

try_install() {
    local name="$1"
    local check_cmd="$2"
    local install_cmd="$3"
    local pkg_mgr="${4:-}"

    # Check if already installed
    if eval "$check_cmd" &>/dev/null; then
        skip "$name (already installed)"
        ((SKIPPED++))
        return 0
    fi

    if $CHECK_ONLY; then
        fail "$name (not installed)"
        ((FAILED++))
        FAILED_LIST+=("$name")
        return 0
    fi

    # Check if the package manager is available
    if [[ -n "$pkg_mgr" ]] && ! has "$pkg_mgr"; then
        fail "$name (requires $pkg_mgr — not found)"
        ((FAILED++))
        FAILED_LIST+=("$name")
        return 0
    fi

    info "Installing $name..."
    if eval "$install_cmd" 2>&1; then
        ok "$name installed"
        ((INSTALLED++))
    else
        fail "$name — install command failed"
        ((FAILED++))
        FAILED_LIST+=("$name")
    fi
}

# Download a JAR tool and create a wrapper script in ~/.local/bin or ~/bin.
install_jar_tool() {
    local name="$1"
    local download_url="$2"
    local check_cmd="$3"

    # Check if already installed
    if eval "$check_cmd" &>/dev/null; then
        skip "$name (already installed)"
        ((SKIPPED++))
        return 0
    fi

    if $CHECK_ONLY; then
        fail "$name (not installed)"
        ((FAILED++))
        FAILED_LIST+=("$name")
        return 0
    fi

    if ! has java; then
        fail "$name (requires java — not found)"
        ((FAILED++))
        FAILED_LIST+=("$name")
        return 0
    fi

    mkdir -p "$TOOLS_DIR"
    local jar_path="$TOOLS_DIR/$name.jar"

    info "Downloading $name JAR..."
    if ! curl -fsSL "$download_url" -o "$jar_path"; then
        fail "$name — download failed"
        ((FAILED++))
        FAILED_LIST+=("$name")
        return 0
    fi

    # Create wrapper in ~/.local/bin (standard location)
    local bin_dir="$HOME/.local/bin"
    mkdir -p "$bin_dir"

    cat > "$bin_dir/$name" << WRAPPER
#!/bin/sh
java -jar "\$HOME/.rice-guard/tools/$name.jar" "\$@"
WRAPPER
    chmod +x "$bin_dir/$name"

    # Ensure ~/.local/bin is in PATH
    if [[ ":$PATH:" != *":$bin_dir:"* ]]; then
        export PATH="$bin_dir:$PATH"
        warn "Added $bin_dir to PATH for this session"
        info "Add to your shell profile: export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi

    ok "$name installed (JAR + wrapper in $bin_dir)"
    ((INSTALLED++))
}

# ── Package manager detection ───────────────────────────────────────────────
HAS_BREW=false;  has brew  && HAS_BREW=true
HAS_GO=false;    has go    && HAS_GO=true
HAS_PIP=false;   has pip3  && HAS_PIP=true || { has pip && HAS_PIP=true; }
HAS_NPM=false;   has npm   && HAS_NPM=true
HAS_GEM=false;   has gem   && HAS_GEM=true
HAS_CARGO=false; has cargo && HAS_CARGO=true
HAS_JAVA=false;  has java  && HAS_JAVA=true

PIP_CMD="pip3"; has pip3 || PIP_CMD="pip"

section "Package Manager Status"
echo -e "  brew:  $($HAS_BREW  && echo -e "${GREEN}found${NC}" || echo -e "${RED}not found${NC}")"
echo -e "  go:    $($HAS_GO    && echo -e "${GREEN}found${NC}" || echo -e "${RED}not found${NC}")"
echo -e "  pip:   $($HAS_PIP   && echo -e "${GREEN}found${NC}" || echo -e "${RED}not found${NC}")"
echo -e "  npm:   $($HAS_NPM   && echo -e "${GREEN}found${NC}" || echo -e "${RED}not found${NC}")"
echo -e "  gem:   $($HAS_GEM   && echo -e "${GREEN}found${NC}" || echo -e "${RED}not found${NC}")"
echo -e "  cargo: $($HAS_CARGO && echo -e "${GREEN}found${NC}" || echo -e "${RED}not found${NC}")"
echo -e "  java:  $($HAS_JAVA  && echo -e "${GREEN}found${NC}" || echo -e "${RED}not found${NC}")"

# ── JAVA_HOME validation ────────────────────────────────────────────────────
if $HAS_JAVA && [[ -n "${JAVA_HOME:-}" ]] && [[ ! -d "$JAVA_HOME" ]]; then
    warn "JAVA_HOME points to non-existent directory: $JAVA_HOME"
    JAVA_BIN="$(command -v java)"
    JDK_HOME="$(dirname "$(dirname "$(readlink -f "$JAVA_BIN")")")"
    if [[ -d "$JDK_HOME" ]]; then
        info "Auto-fixing JAVA_HOME to: $JDK_HOME"
        export JAVA_HOME="$JDK_HOME"
    fi
fi

# ── Composer global bin PATH ─────────────────────────────────────────────────
if has composer; then
    COMPOSER_BIN="$(composer global config bin-dir --absolute 2>/dev/null | tr -d '\r' || true)"
    if [[ -n "$COMPOSER_BIN" && -d "$COMPOSER_BIN" && ":$PATH:" != *":$COMPOSER_BIN:"* ]]; then
        export PATH="$PATH:$COMPOSER_BIN"
        info "Added composer global bin to PATH: $COMPOSER_BIN"
    fi
fi

# ── .NET global tools PATH ─────────────────────────────────────────────────
DOTNET_TOOLS_DIR="$HOME/.dotnet/tools"
if [[ -d "$DOTNET_TOOLS_DIR" && ":$PATH:" != *":$DOTNET_TOOLS_DIR:"* ]]; then
    export PATH="$PATH:$DOTNET_TOOLS_DIR"
    info "Added .NET global tools to PATH: $DOTNET_TOOLS_DIR"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# SCANNERS
# ═══════════════════════════════════════════════════════════════════════════════
install_scanners() {
    section "Scanners"

    # semgrep — Python-based SAST
    if $HAS_PIP; then
        try_install "semgrep" "semgrep --version" "$PIP_CMD install semgrep" "$PIP_CMD"
    elif $HAS_BREW; then
        try_install "semgrep" "semgrep --version" "brew install semgrep" "brew"
    else
        fail "semgrep (needs pip or brew)"
        ((FAILED++)); FAILED_LIST+=("semgrep")
    fi

    # trivy — vulnerability scanner
    if $HAS_BREW; then
        try_install "trivy" "trivy --version" "brew install trivy" "brew"
    else
        try_install "trivy" "trivy --version" \
            "curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -s -- -b /usr/local/bin"
    fi

    # gitleaks — secret scanner
    if $HAS_BREW; then
        try_install "gitleaks" "gitleaks version" "brew install gitleaks" "brew"
    else
        try_install "gitleaks" "gitleaks version" \
            "curl -sfL https://raw.githubusercontent.com/gitleaks/gitleaks/master/scripts/install.sh | sh -s -- -b /usr/local/bin"
    fi

    # jscpd — copy/paste detector
    try_install "jscpd" "jscpd --version" "npm install -g jscpd" "npm"

    # scc — code counter + language detection
    if $HAS_BREW; then
        try_install "scc" "scc --version" "brew install scc" "brew"
    elif $HAS_GO; then
        try_install "scc" "scc --version" "go install github.com/boyter/scc/v3@latest" "go"
    else
        fail "scc (needs brew or go)"
        ((FAILED++)); FAILED_LIST+=("scc")
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Go
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_go() {
    section "Fixers — Go"
    try_install "gofumpt"        "gofumpt --version"        "go install mvdan.cc/gofumpt@latest"                                   "go"
    try_install "goimports"      "which goimports"           "go install golang.org/x/tools/cmd/goimports@latest"                   "go"
    try_install "golangci-lint"  "golangci-lint --version"   "go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest" "go"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Python
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_python() {
    section "Fixers — Python"
    try_install "ruff"       "ruff --version"       "$PIP_CMD install ruff"      "$PIP_CMD"
    try_install "pip-audit"  "pip-audit --version"  "$PIP_CMD install pip-audit" "$PIP_CMD"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — JavaScript / TypeScript
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_js() {
    section "Fixers — JavaScript / TypeScript"
    try_install "biome"     "biome --version"     "npm install -g @biomejs/biome" "npm"
    try_install "prettier"  "prettier --version"  "npm install -g prettier"       "npm"
    try_install "eslint"    "eslint --version"    "npm install -g eslint"         "npm"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Rust
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_rust() {
    section "Fixers — Rust"
    try_install "rustfmt"       "rustfmt --version"     "rustup component add rustfmt"  "rustup"
    try_install "clippy"        "cargo clippy --version" "rustup component add clippy"   "rustup"
    try_install "cargo-outdated" "cargo outdated --version" "cargo install cargo-outdated" "cargo"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Ruby
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_ruby() {
    section "Fixers — Ruby"
    try_install "rubocop"       "rubocop --version"       "gem install rubocop"       "gem"
    try_install "bundler-audit" "bundle-audit version"    "gem install bundler-audit" "gem"
    try_install "brakeman"      "brakeman --version"      "gem install brakeman"      "gem"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Java
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_java() {
    section "Fixers — Java"

    # google-java-format: try brew, fall back to JAR
    if $HAS_BREW; then
        try_install "google-java-format" "google-java-format --version" "brew install google-java-format" "brew"
    else
        install_jar_tool "google-java-format" \
            "https://github.com/google/google-java-format/releases/latest/download/google-java-format-all-deps.jar" \
            "google-java-format --version"
    fi

    # checkstyle: JAR download (no reliable brew/apt package)
    install_jar_tool "checkstyle" \
        "https://github.com/checkstyle/checkstyle/releases/download/checkstyle-13.3.0/checkstyle-13.3.0-all.jar" \
        "checkstyle --version"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Kotlin
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_kotlin() {
    section "Fixers — Kotlin"

    # detekt: brew or JAR
    if $HAS_BREW; then
        try_install "detekt"  "detekt --version"  "brew install detekt"  "brew"
    else
        install_jar_tool "detekt" \
            "https://github.com/detekt/detekt/releases/latest/download/detekt-cli-all.jar" \
            "detekt --version"
    fi

    # ktfmt: JAR download (no brew formula)
    install_jar_tool "ktfmt" \
        "https://github.com/facebook/ktfmt/releases/download/v0.61/ktfmt-0.61-with-dependencies.jar" \
        "ktfmt --version"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — PHP
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_php() {
    section "Fixers — PHP"

    if ! has php; then
        info "PHP not found. Install via your package manager (brew install php, apt install php)"
        ((FAILED+=2)); FAILED_LIST+=("php-cs-fixer" "phpstan")
        return
    fi

    # Check openssl extension
    if ! php -m 2>/dev/null | grep -qi openssl; then
        warn "PHP openssl extension not loaded"
        info "Enable openssl in your php.ini: extension=openssl"
        ((FAILED+=2)); FAILED_LIST+=("php-cs-fixer" "phpstan")
        return
    fi

    if ! has composer; then
        info "Composer not found. Install from https://getcomposer.org/download/"
        ((FAILED+=2)); FAILED_LIST+=("php-cs-fixer" "phpstan")
        return
    fi

    try_install "php-cs-fixer" "php-cs-fixer --version" "composer global require friendsofphp/php-cs-fixer" "composer"
    try_install "phpstan"      "phpstan --version"      "composer global require phpstan/phpstan"             "composer"
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Shell
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_shell() {
    section "Fixers — Shell"
    if $HAS_GO; then
        try_install "shfmt" "shfmt --version" "go install mvdan.cc/sh/v3/cmd/shfmt@latest" "go"
    elif $HAS_BREW; then
        try_install "shfmt" "shfmt --version" "brew install shfmt" "brew"
    else
        fail "shfmt (needs go or brew)"
        ((FAILED++)); FAILED_LIST+=("shfmt")
    fi

    if $HAS_BREW; then
        try_install "shellcheck" "shellcheck --version" "brew install shellcheck" "brew"
    else
        try_install "shellcheck" "shellcheck --version" "apt-get install -y shellcheck" "apt-get"
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — Dart / Flutter
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_dart() {
    section "Fixers — Dart / Flutter"
    if has dart; then
        ok "dart SDK found (includes dart format, dart fix, dart analyze)"
    else
        info "Install the Dart SDK: https://dart.dev/get-dart"
        ((FAILED++)); FAILED_LIST+=("dart-sdk")
    fi
    if has flutter; then
        ok "flutter SDK found (includes flutter analyze)"
    else
        info "Install Flutter SDK: https://flutter.dev/docs/get-started/install"
        ((FAILED++)); FAILED_LIST+=("flutter-sdk")
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIXERS — C# / .NET
# ═══════════════════════════════════════════════════════════════════════════════
install_fixers_csharp() {
    section "Fixers — C# / .NET"
    if has dotnet; then
        # Check if SDK is installed (not just runtime)
        local sdks
        sdks="$(dotnet --list-sdks 2>&1 || true)"
        if [[ -z "$sdks" || "$sdks" == *"No SDKs"* ]]; then
            warn "dotnet runtime found but no SDK installed"
            info "Install .NET SDK for dotnet format + dotnet-outdated: https://dotnet.microsoft.com/download"
            ((FAILED++)); FAILED_LIST+=("dotnet-sdk")
        else
            ok "dotnet SDK found (includes dotnet format)"
            try_install "dotnet-outdated" "dotnet outdated --version" \
                "dotnet tool install -g dotnet-outdated-tool" "dotnet"
        fi
    else
        info "Install .NET SDK: https://dotnet.microsoft.com/download"
        ((FAILED++)); FAILED_LIST+=("dotnet-sdk")
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════════════

echo -e "${CYAN}rice-guard — Tool Dependency Installer${NC}"
echo -e "Mode: ${YELLOW}${MODE}${NC}  Lang: ${YELLOW}${LANG_FILTER:-all}${NC}  Check-only: ${YELLOW}${CHECK_ONLY}${NC}"

if [[ "$MODE" == "all" || "$MODE" == "scanners" ]]; then
    install_scanners
fi

if [[ "$MODE" == "all" || "$MODE" == "fixers" ]]; then
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "go" ]];         then install_fixers_go; fi
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "python" ]];     then install_fixers_python; fi
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "javascript" || "$LANG_FILTER" == "typescript" || "$LANG_FILTER" == "js" || "$LANG_FILTER" == "ts" ]]; then
        install_fixers_js
    fi
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "rust" ]];       then install_fixers_rust; fi
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "ruby" ]];       then install_fixers_ruby; fi
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "java" ]];       then install_fixers_java; fi
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "kotlin" ]];     then install_fixers_kotlin; fi
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "php" ]];        then install_fixers_php; fi
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "shell" || "$LANG_FILTER" == "bash" ]]; then
        install_fixers_shell
    fi
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "dart" || "$LANG_FILTER" == "flutter" ]]; then
        install_fixers_dart
    fi
    if [[ -z "$LANG_FILTER" || "$LANG_FILTER" == "csharp" || "$LANG_FILTER" == "dotnet" ]]; then
        install_fixers_csharp
    fi
fi

# ── Summary ──────────────────────────────────────────────────────────────────
section "Summary"
echo -e "  ${GREEN}Installed:${NC} $INSTALLED"
echo -e "  ${YELLOW}Skipped:${NC}   $SKIPPED (already installed)"
echo -e "  ${RED}Failed:${NC}    $FAILED"

if [[ ${#FAILED_LIST[@]} -gt 0 ]]; then
    echo -e "\n${RED}Failed tools:${NC}"
    for tool in "${FAILED_LIST[@]}"; do
        echo -e "  - $tool"
    done
fi

if [[ $FAILED -gt 0 ]]; then
    exit 1
fi
