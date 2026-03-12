#!/usr/bin/env bash
# rice-guard installer — detects OS/arch and downloads the correct binary.
set -euo pipefail

REPO="user/rice-guard"  # TODO: update to real GitHub owner
VERSION="${RICE_GUARD_VERSION:-latest}"
INSTALL_DIR="${RICE_GUARD_INSTALL_DIR:-$HOME/.local/bin}"

detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64)  echo "x86_64-unknown-linux-musl" ;;
                aarch64) echo "aarch64-unknown-linux-musl" ;;
                *)       echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64)  echo "x86_64-apple-darwin" ;;
                arm64)   echo "aarch64-apple-darwin" ;;
                *)       echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        *)
            echo "Unsupported OS: $os (use install.ps1 for Windows)" >&2
            exit 1
            ;;
    esac
}

get_download_url() {
    local target="$1"
    if [ "$VERSION" = "latest" ]; then
        echo "https://github.com/$REPO/releases/latest/download/rice-guard-${target}.tar.gz"
    else
        echo "https://github.com/$REPO/releases/download/v${VERSION}/rice-guard-${target}.tar.gz"
    fi
}

main() {
    local target url tmpdir
    target="$(detect_target)"
    url="$(get_download_url "$target")"

    echo "Installing rice-guard for $target..."
    echo "  From: $url"
    echo "  To:   $INSTALL_DIR/rice-guard"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    curl -fsSL "$url" -o "$tmpdir/rice-guard.tar.gz"
    tar xzf "$tmpdir/rice-guard.tar.gz" -C "$tmpdir"

    mkdir -p "$INSTALL_DIR"
    mv "$tmpdir/rice-guard" "$INSTALL_DIR/rice-guard"
    chmod +x "$INSTALL_DIR/rice-guard"

    echo ""
    if command -v rice-guard &>/dev/null; then
        echo "Installed: $(rice-guard version 2>/dev/null || echo 'rice-guard')"
    else
        echo "Installed to $INSTALL_DIR/rice-guard"
        echo "Add $INSTALL_DIR to your PATH if not already present."
    fi
}

main
