#!/usr/bin/env bash
# First-time installation of fuel-logger-rs on a fresh server.
# Usage: sudo ./install.sh
set -euo pipefail

REPO="holoujak/fuel-logger-rs"
INSTALL_DIR="/opt/fuel-logger-rs"
SERVICE_NAME="fuel-logger-rs"
SERVICE_USER="fuel-logger"

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64)  ASSET_NAME="fuel-logger-rs-x86_64" ;;
  aarch64) ASSET_NAME="fuel-logger-rs-aarch64" ;;
  *)       echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "🚀 Installing $SERVICE_NAME ($ASSET_NAME)..."

# 1. Create system user (no login shell, no home)
if ! id "$SERVICE_USER" &>/dev/null; then
  echo "📦 Creating system user: $SERVICE_USER"
  useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
fi

# On Raspberry Pi, add user to gpio group for hardware access
if getent group gpio &>/dev/null; then
  usermod -aG gpio "$SERVICE_USER"
fi

# 2. Create install directory
mkdir -p "$INSTALL_DIR"

# 3. Download latest release binary
echo "⬇️  Downloading latest release from GitHub..."
DOWNLOAD_URL=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep -o "https://github.com/${REPO}/releases/download/[^\"]*/${ASSET_NAME}" \
  | head -1)

if [ -z "$DOWNLOAD_URL" ]; then
  echo "❌ Could not find release asset: $ASSET_NAME"
  exit 1
fi

curl -fsSL -o "${INSTALL_DIR}/fuel-logger-rs" "$DOWNLOAD_URL"
chmod +x "${INSTALL_DIR}/fuel-logger-rs"

# 4. Copy example config if no config exists
if [ ! -f "${INSTALL_DIR}/config.toml" ]; then
  if [ -f "config.toml.example" ]; then
    cp config.toml.example "${INSTALL_DIR}/config.toml"
    echo "📝 Copied config.toml.example → ${INSTALL_DIR}/config.toml"
    echo "   ⚠️  Edit ${INSTALL_DIR}/config.toml before starting!"
  fi
fi

# 5. Set ownership
chown -R "$SERVICE_USER":"$SERVICE_USER" "$INSTALL_DIR"

# 6. Copy update script to install dir for easy future updates
cp deploy/update.sh "${INSTALL_DIR}/update.sh"
chmod +x "${INSTALL_DIR}/update.sh"

# 7. Install systemd service
cp deploy/fuel-logger-rs.service /etc/systemd/system/${SERVICE_NAME}.service
systemctl daemon-reload
systemctl enable "$SERVICE_NAME"

echo ""
echo "✅ Installation complete!"
echo ""
echo "Next steps:"
echo "  1. Edit config:    sudo nano ${INSTALL_DIR}/config.toml"
echo "  2. Start service:  sudo systemctl start ${SERVICE_NAME}"
echo "  3. Check status:   sudo systemctl status ${SERVICE_NAME}"
echo "  4. View logs:      sudo journalctl -u ${SERVICE_NAME} -f"
