#!/usr/bin/env bash
# rguard installer — detects OS/arch and downloads the correct binary.
set -euo pipefail

REPO="moabualruz/rice-guard"  # TODO: update to real GitHub owner
VERSION="${RGUARD_VERSION:-latest}"
INSTALL_DIR="${RGUARD_INSTALL_DIR:-$HOME/.local/bin}"

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
        echo "https://github.com/$REPO/releases/latest/download/rguard-${target}.tar.gz"
    else
        echo "https://github.com/$REPO/releases/download/v${VERSION}/rguard-${target}.tar.gz"
    fi
}

main() {
    local target url tmpdir
    target="$(detect_target)"
    url="$(get_download_url "$target")"

    echo "Installing rguard for $target..."
    echo "  From: $url"
    echo "  To:   $INSTALL_DIR/rguard"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    curl -fsSL "$url" -o "$tmpdir/rguard.tar.gz"
    tar xzf "$tmpdir/rguard.tar.gz" -C "$tmpdir"

    mkdir -p "$INSTALL_DIR"
    mv "$tmpdir/rguard" "$INSTALL_DIR/rguard"
    chmod +x "$INSTALL_DIR/rguard"

    echo ""
    if command -v rguard &>/dev/null; then
        echo "Installed: $(rguard version 2>/dev/null || echo 'rguard')"
    else
        echo "Installed to $INSTALL_DIR/rguard"
        echo "Add $INSTALL_DIR to your PATH if not already present."
    fi
}

main
