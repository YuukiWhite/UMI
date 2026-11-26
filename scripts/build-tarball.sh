#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-$(awk -F'"' '/^version =/{print $2; exit}' "$ROOT_DIR/Cargo.toml")}"
BIN_SRC="$ROOT_DIR/target/release/umi"
DIST_DIR="$ROOT_DIR/dist"
STAGE_NAME="umi-${VERSION}-x86_64-linux"
STAGE="$DIST_DIR/$STAGE_NAME"
OUT_TAR="$DIST_DIR/UMI-${VERSION}-x86_64-linux.tar.gz"

if [[ ! -x "$BIN_SRC" ]]; then
  (cd "$ROOT_DIR" && cargo build --release --locked)
fi

rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/lib" "$STAGE/assets/icons"
install -m 0755 "$BIN_SRC" "$STAGE/bin/umi-bin"
LAYER_SHELL_SO="$(ldd "$BIN_SRC" | awk '/libgtk4-layer-shell\.so/ {print $3; exit}')"
if [[ -n "${LAYER_SHELL_SO:-}" && -f "$LAYER_SHELL_SO" ]]; then
  cp -L "$LAYER_SHELL_SO" "$STAGE/lib/libgtk4-layer-shell.so.0"
  chmod 0755 "$STAGE/lib/libgtk4-layer-shell.so.0"
fi
install -m 0644 "$ROOT_DIR/assets/icons/umi-logo.svg" "$STAGE/assets/icons/umi-logo.svg"
install -m 0644 "$ROOT_DIR/README.md" "$STAGE/README.md"

# Self-contained offline installer bundled inside the tarball
cat > "$STAGE/install.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
LIB_DIR="${HOME}/.local/lib/umi"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
QUOTA_DIR="${HOME}/.config/umi/quota-scripts"

mkdir -p "$BIN_DIR" "$LIB_DIR" "$APP_DIR" "$ICON_DIR" "$QUOTA_DIR"
install -m 0755 "$DIR/bin/umi-bin" "$LIB_DIR/umi"
if [[ -f "$DIR/lib/libgtk4-layer-shell.so.0" ]]; then
  install -m 0755 "$DIR/lib/libgtk4-layer-shell.so.0" "$LIB_DIR/libgtk4-layer-shell.so.0"
fi
cat > "$BIN_DIR/umi" <<WRAP
#!/usr/bin/env bash
export LD_LIBRARY_PATH="${LIB_DIR}\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH}"
exec "${LIB_DIR}/umi" "\$@"
WRAP
chmod 0755 "$BIN_DIR/umi"
install -m 0644 "$DIR/assets/icons/umi-logo.svg" "$ICON_DIR/org.umi.Notch.svg"

cat > "$APP_DIR/org.umi.Notch.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=UMI
GenericName=Ambient Notch Companion
Comment=Dynamic Island desktop overlay for Linux Wayland (KDE Plasma)
Exec=${BIN_DIR}/umi
Icon=org.umi.Notch
Terminal=false
Categories=Utility;
StartupNotify=false
StartupWMClass=org.umi.Notch
DESKTOP

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
command -v gtk4-update-icon-cache >/dev/null 2>&1 && gtk4-update-icon-cache -q "${HOME}/.local/share/icons/hicolor" || true

echo "UMI installed to ${BIN_DIR}/umi"
echo "Make sure ~/.local/bin is in your PATH, or launch 'UMI' from KDE Kickoff / KRunner."
EOF
chmod 0755 "$STAGE/install.sh"

install -m 0755 "$ROOT_DIR/scripts/uninstall.sh" "$STAGE/uninstall.sh"

tar -czf "$OUT_TAR" -C "$DIST_DIR" "$STAGE_NAME"
rm -rf "$STAGE"
echo "Built $OUT_TAR"
ls -lh "$OUT_TAR"
