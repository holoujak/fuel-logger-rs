#!/usr/bin/env bash
# Update fuel-logger-rs to the latest (or specified) GitHub release.
# Usage:
#   sudo ./update.sh            # latest release
#   sudo ./update.sh v1.2.3     # specific tag
set -euo pipefail

REPO="holoujak/fuel-logger-rs"
INSTALL_DIR="/opt/fuel-logger-rs"
SERVICE_NAME="fuel-logger-rs"
BINARY="${INSTALL_DIR}/fuel-logger-rs"
TAG="${1:-latest}"

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64)  ASSET_NAME="fuel-logger-rs-x86_64" ;;
  aarch64) ASSET_NAME="fuel-logger-rs-aarch64" ;;
  *)       echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Resolve download URL
if [ "$TAG" = "latest" ]; then
  API_URL="https://api.github.com/repos/${REPO}/releases/latest"
else
  API_URL="https://api.github.com/repos/${REPO}/releases/tags/${TAG}"
fi

echo "🔍 Fetching release info ($TAG)..."
RELEASE_JSON=$(curl -fsSL "$API_URL")
RELEASE_TAG=$(echo "$RELEASE_JSON" | grep -o '"tag_name": *"[^"]*"' | head -1 | cut -d'"' -f4)

DOWNLOAD_URL=$(echo "$RELEASE_JSON" \
  | grep -o "https://github.com/${REPO}/releases/download/[^\"]*/${ASSET_NAME}" \
  | head -1)

if [ -z "$DOWNLOAD_URL" ]; then
  echo "❌ Could not find asset '$ASSET_NAME' in release '$TAG'"
  exit 1
fi

echo "📦 Release: $RELEASE_TAG"
echo "⬇️  Downloading $ASSET_NAME..."

# Download to temp file first
TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT
curl -fsSL -o "$TMPFILE" "$DOWNLOAD_URL"
chmod +x "$TMPFILE"

# Quick sanity check – make sure it's an ELF binary
if ! file "$TMPFILE" | grep -q "ELF"; then
  echo "❌ Downloaded file is not a valid ELF binary!"
  exit 1
fi

# Stop service, swap binary, start service
echo "🔄 Stopping ${SERVICE_NAME}..."
systemctl stop "$SERVICE_NAME"

# Backup current binary
if [ -f "$BINARY" ]; then
  cp "$BINARY" "${BINARY}.bak"
fi

mv "$TMPFILE" "$BINARY"
chown fuel-logger:fuel-logger "$BINARY"

echo "🚀 Starting ${SERVICE_NAME}..."
systemctl start "$SERVICE_NAME"

# Verify it came up
sleep 2
if systemctl is-active --quiet "$SERVICE_NAME"; then
  echo "✅ Updated to $RELEASE_TAG and running!"
else
  echo "⚠️  Service failed to start! Rolling back..."
  if [ -f "${BINARY}.bak" ]; then
    mv "${BINARY}.bak" "$BINARY"
    systemctl start "$SERVICE_NAME"
    echo "🔙 Rolled back to previous version."
  fi
  echo "Check logs: sudo journalctl -u ${SERVICE_NAME} -n 50"
  exit 1
fi

# Clean up backup on success
rm -f "${BINARY}.bak"
