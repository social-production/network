#!/usr/bin/env bash
# install.sh — Build and install the Social Production Network node (sp-node)
#
# Supported platforms: Linux, macOS
# For Windows use scripts/install.ps1 instead.
#
# Usage:
#   ./scripts/install.sh [--prefix <dir>]
#
# Options:
#   --prefix <dir>   Install root.  Defaults to /usr/local.
#                    The binary is placed in <prefix>/bin/sp-node.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(dirname "$SCRIPT_DIR")"
PREFIX="/usr/local"

# ── Detect OS ──────────────────────────────────────────────────────────────────
OS="$(uname -s)"
case "$OS" in
    Linux*)  PLATFORM="linux" ;;
    Darwin*) PLATFORM="macos" ;;
    *)
        echo "Unsupported platform: $OS" >&2
        echo "For Windows, use scripts/install.ps1" >&2
        exit 1
        ;;
esac

# ── Parse arguments ────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [--prefix <dir>]"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

BIN_DIR="$PREFIX/bin"

# ── Build ──────────────────────────────────────────────────────────────────────
echo "==> Building sp-node (release) on $OS…"
cargo build --release --manifest-path "$WORKSPACE_DIR/Cargo.toml" -p sp-node

BINARY="$WORKSPACE_DIR/target/release/sp-node"
if [[ ! -f "$BINARY" ]]; then
    echo "Build succeeded but binary not found at $BINARY" >&2
    exit 1
fi

# ── Install binary ─────────────────────────────────────────────────────────────
echo "==> Installing sp-node to $BIN_DIR…"
mkdir -p "$BIN_DIR"
cp "$BINARY" "$BIN_DIR/sp-node"
chmod 755 "$BIN_DIR/sp-node"
echo "    Installed: $BIN_DIR/sp-node"

# ── Platform-specific service installation ─────────────────────────────────────
if [[ "$PLATFORM" == "linux" ]]; then
    SYSTEMD_DIR="/etc/systemd/system"
    SERVICE_FILE="$SCRIPT_DIR/sp-node.service"

    if command -v systemctl &>/dev/null && [[ -d "$SYSTEMD_DIR" ]]; then
        if [[ -f "$SERVICE_FILE" ]]; then
            if [[ $EUID -eq 0 ]]; then
                echo "==> Installing systemd service…"
                cp "$SERVICE_FILE" "$SYSTEMD_DIR/sp-node.service"
                chmod 644 "$SYSTEMD_DIR/sp-node.service"
                systemctl daemon-reload
                echo "    Installed: $SYSTEMD_DIR/sp-node.service"
                echo "    Enable and start with:"
                echo "      systemctl enable --now sp-node"
            else
                echo "    (Skipping systemd install — run as root to install the service)"
                echo "    You can install it manually:"
                echo "      sudo cp $SERVICE_FILE $SYSTEMD_DIR/sp-node.service"
                echo "      sudo systemctl daemon-reload"
                echo "      sudo systemctl enable --now sp-node"
            fi
        fi
    fi

elif [[ "$PLATFORM" == "macos" ]]; then
    PLIST_SRC="$SCRIPT_DIR/com.socialproduction.spnode.plist"
    LAUNCH_AGENTS="$HOME/Library/LaunchAgents"
    PLIST_DST="$LAUNCH_AGENTS/com.socialproduction.spnode.plist"

    if [[ -f "$PLIST_SRC" ]]; then
        echo "==> Installing launchd service…"
        mkdir -p "$LAUNCH_AGENTS"
        # Substitute the actual binary path into the plist.
        sed "s|INSTALL_PREFIX|$PREFIX|g" "$PLIST_SRC" > "$PLIST_DST"
        chmod 644 "$PLIST_DST"
        echo "    Installed: $PLIST_DST"
        echo "    Load and start with:"
        echo "      launchctl load $PLIST_DST"
        echo "    Or to start automatically at login it is already set (RunAtLoad=true)."
    fi
fi

echo ""
echo "Installation complete.  Run 'sp-node --help' to get started."
