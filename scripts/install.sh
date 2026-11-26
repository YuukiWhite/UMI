#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SRC="$ROOT_DIR/target/release/umi"
BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
QUOTA_DIR="${HOME}/.config/umi/quota-scripts"

if [[ ! -x "$BIN_SRC" ]]; then
  echo "Release binary not found at $BIN_SRC — building..."
  (cd "$ROOT_DIR" && cargo build --release --offline)
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR" "$QUOTA_DIR"
install -m 0755 "$BIN_SRC" "$BIN_DIR/umi"
install -m 0644 "$ROOT_DIR/assets/icons/umi-logo.svg" "$ICON_DIR/org.umi.Notch.svg"

cat > "$APP_DIR/org.umi.Notch.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=UMI
GenericName=Ambient Notch Companion
Comment=Dynamic Island desktop overlay for KDE Plasma Wayland
Exec=${BIN_DIR}/umi
Icon=org.umi.Notch
Terminal=false
Categories=Utility;
StartupNotify=false
StartupWMClass=org.umi.Notch
EOF

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
command -v gtk4-update-icon-cache >/dev/null 2>&1 && gtk4-update-icon-cache -q "${HOME}/.local/share/icons/hicolor" || true

echo "Installed UMI to ${BIN_DIR}/umi"
echo "Launcher installed at ${APP_DIR}/org.umi.Notch.desktop"
echo "Run 'umi' from anywhere or launch 'UMI' from KDE Kickoff / KRunner."
