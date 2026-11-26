#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-$(awk -F'"' '/^version =/{print $2; exit}' "$ROOT_DIR/Cargo.toml")}"
BIN_SRC="$ROOT_DIR/target/release/umi"
DIST_DIR="$ROOT_DIR/dist"
STAGE="$DIST_DIR/umi_${VERSION}_amd64"
OUT_DEB="$DIST_DIR/umi_${VERSION}_amd64.deb"

if [[ ! -x "$BIN_SRC" ]]; then
  (cd "$ROOT_DIR" && cargo build --release --locked)
fi

rm -rf "$STAGE"
mkdir -p \
  "$STAGE/DEBIAN" \
  "$STAGE/usr/bin" \
  "$STAGE/usr/lib/umi" \
  "$STAGE/usr/share/applications" \
  "$STAGE/usr/share/icons/hicolor/scalable/apps"

install -m 0755 "$BIN_SRC" "$STAGE/usr/lib/umi/umi"
LAYER_SHELL_SO="$(ldd "$BIN_SRC" | awk '/libgtk4-layer-shell\.so/ {print $3; exit}')"
if [[ -n "${LAYER_SHELL_SO:-}" && -f "$LAYER_SHELL_SO" ]]; then
  cp -L "$LAYER_SHELL_SO" "$STAGE/usr/lib/umi/libgtk4-layer-shell.so.0"
  chmod 0755 "$STAGE/usr/lib/umi/libgtk4-layer-shell.so.0"
fi
cat > "$STAGE/usr/bin/umi" <<'EOF'
#!/usr/bin/env bash
export LD_LIBRARY_PATH="/usr/lib/umi${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec /usr/lib/umi/umi "$@"
EOF
chmod 0755 "$STAGE/usr/bin/umi"
install -m 0644 "$ROOT_DIR/assets/icons/umi-logo.svg" "$STAGE/usr/share/icons/hicolor/scalable/apps/org.umi.Notch.svg"

cat > "$STAGE/usr/share/applications/org.umi.Notch.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=UMI
GenericName=Ambient Notch Companion
Comment=Dynamic Island desktop overlay for Linux Wayland (KDE Plasma)
Exec=/usr/bin/umi
Icon=org.umi.Notch
Terminal=false
Categories=Utility;DesktopUtility;
StartupNotify=false
StartupWMClass=org.umi.Notch
EOF

cat > "$STAGE/DEBIAN/control" <<EOF
Package: umi
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: YuukiWhite <YuukiWhite@users.noreply.github.com>
Depends: libgtk-4-1, libsqlite3-0, libdbus-1-3
Homepage: https://github.com/YuukiWhite/UMI
Description: UMI — Ambient Dynamic Island Companion for Linux Wayland
 Pure Rust GTK4 layer-shell Dynamic Island overlay with 18 interactive states,
 MPRIS2 + LRCLIB synced lyrics, offline Prayer Times, Pomodoro, Hydration,
 File Shelf DnD, Ask UMI AI + Gemini Vision OCR, live Codex quota tracker,
 and 7-screen Desktop Settings.
EOF

dpkg-deb --root-owner-group --build "$STAGE" "$OUT_DEB"
rm -rf "$STAGE"
echo "Built $OUT_DEB"
ls -lh "$OUT_DEB"
