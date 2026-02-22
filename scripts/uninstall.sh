#!/usr/bin/env bash
# uninstall.sh — Remove the Social Production Network node (sp-node)
#
# Supported platforms: Linux, macOS
# For Windows use scripts/uninstall.ps1 instead.
#
# Usage:
#   ./scripts/uninstall.sh [--prefix <dir>]
#
# Options:
#   --prefix <dir>   Install root used during installation.  Defaults to /usr/local.

set -euo pipefail

PREFIX="/usr/local"

# ── Detect OS ──────────────────────────────────────────────────────────────────
OS="$(uname -s)"
case "$OS" in
    Linux*)  PLATFORM="linux" ;;
    Darwin*) PLATFORM="macos" ;;
    *)
        echo "Unsupported platform: $OS" >&2
        echo "For Windows, use scripts/uninstall.ps1" >&2
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

BIN="$PREFIX/bin/sp-node"

# ── Platform-specific service removal ─────────────────────────────────────────
if [[ "$PLATFORM" == "linux" ]]; then
    SYSTEMD_DIR="/etc/systemd/system"
    SERVICE="$SYSTEMD_DIR/sp-node.service"

    if command -v systemctl &>/dev/null && [[ -f "$SERVICE" ]]; then
        if [[ $EUID -eq 0 ]]; then
            echo "==> Stopping and disabling sp-node service…"
            systemctl stop sp-node 2>/dev/null || true
            systemctl disable sp-node 2>/dev/null || true
            rm -f "$SERVICE"
            systemctl daemon-reload
            echo "    Removed: $SERVICE"
        else
            echo "    (Skipping systemd removal — run as root to remove the service)"
            echo "    You can remove it manually:"
            echo "      sudo systemctl stop sp-node"
            echo "      sudo systemctl disable sp-node"
            echo "      sudo rm $SERVICE"
            echo "      sudo systemctl daemon-reload"
        fi
    fi

elif [[ "$PLATFORM" == "macos" ]]; then
    PLIST="$HOME/Library/LaunchAgents/com.socialproduction.spnode.plist"

    if [[ -f "$PLIST" ]]; then
        echo "==> Stopping and removing launchd service…"
        launchctl unload "$PLIST" 2>/dev/null || true
        rm -f "$PLIST"
        echo "    Removed: $PLIST"
    fi
fi

# ── Remove binary ──────────────────────────────────────────────────────────────
if [[ -f "$BIN" ]]; then
    echo "==> Removing $BIN…"
    rm -f "$BIN"
    echo "    Removed: $BIN"
else
    echo "    Binary not found at $BIN (already removed?)"
fi

echo ""
echo "Uninstall complete."
