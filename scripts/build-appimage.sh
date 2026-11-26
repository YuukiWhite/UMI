#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-$(awk -F'"' '/^version =/{print $2; exit}' "$ROOT_DIR/Cargo.toml")}"
BIN_SRC="$ROOT_DIR/target/release/umi"
DIST_DIR="$ROOT_DIR/dist"
APPDIR="$DIST_DIR/UMI.AppDir"
OUT_APPIMAGE="$DIST_DIR/UMI-${VERSION}-x86_64.AppImage"

if [[ ! -x "$BIN_SRC" ]]; then
  echo "Building release binary..."
  (cd "$ROOT_DIR" && cargo build --release --locked)
fi

rm -rf "$APPDIR"
mkdir -p \
  "$APPDIR/usr/bin" \
  "$APPDIR/usr/lib" \
  "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/icons/hicolor/256x256/apps" \
  "$APPDIR/usr/share/icons/hicolor/scalable/apps" \
  "$APPDIR/usr/share/metainfo"

install -m 0755 "$BIN_SRC" "$APPDIR/usr/bin/umi"
install -m 0644 "$ROOT_DIR/assets/icons/umi-logo.png" "$APPDIR/org.umi.Notch.png"
install -m 0644 "$ROOT_DIR/assets/icons/umi-logo.png" "$APPDIR/.DirIcon"
install -m 0644 "$ROOT_DIR/assets/icons/umi-logo.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/org.umi.Notch.png"
install -m 0644 "$ROOT_DIR/assets/icons/umi-logo.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/org.umi.Notch.svg"

# Bundle libgtk4-layer-shell so distros without it preinstalled still run UMI out of the box
LAYER_SHELL_SO="$(ldd "$BIN_SRC" | awk '/libgtk4-layer-shell\.so/ {print $3; exit}')"
if [[ -n "${LAYER_SHELL_SO:-}" && -f "$LAYER_SHELL_SO" ]]; then
  cp -L "$LAYER_SHELL_SO" "$APPDIR/usr/lib/libgtk4-layer-shell.so.0"
  chmod 0755 "$APPDIR/usr/lib/libgtk4-layer-shell.so.0"
  echo "Bundled $LAYER_SHELL_SO -> AppDir/usr/lib/libgtk4-layer-shell.so.0"
fi

cat > "$APPDIR/org.umi.Notch.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=UMI
GenericName=Ambient Notch Companion
Comment=Dynamic Island desktop overlay for Linux Wayland (KDE Plasma)
Exec=umi
Icon=org.umi.Notch
Terminal=false
Categories=Utility;
StartupNotify=false
StartupWMClass=org.umi.Notch
EOF
cp "$APPDIR/org.umi.Notch.desktop" "$APPDIR/usr/share/applications/org.umi.Notch.desktop"

cat > "$APPDIR/usr/share/metainfo/org.umi.Notch.appdata.xml" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>org.umi.Notch</id>
  <name>UMI</name>
  <summary>Ambient Dynamic Island Companion for Linux Wayland</summary>
  <description>
    <p>UMI is a Pure Rust GTK4 layer-shell Dynamic Island desktop overlay for Linux Wayland (KDE Plasma) with 18 interactive notch states, MPRIS2 synced lyrics, offline Prayer Times, Pomodoro, Hydration, File Shelf DnD, Ask UMI AI, live Codex quotas, and 7-screen Settings.</p>
  </description>
  <developer id="io.github.yuukiwhite">
    <name>YuukiWhite</name>
  </developer>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>MIT</project_license>
  <content_rating type="oars-1.1"/>
  <url type="homepage">https://github.com/YuukiWhite/UMI</url>
  <launchable type="desktop-id">org.umi.Notch.desktop</launchable>
  <releases>
    <release version="${VERSION}" date="$(date +%Y-%m-%d)"/>
  </releases>
</component>
EOF

cat > "$APPDIR/AppRun" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
APPDIR="$(dirname "$(readlink -f "$0")")"
export LD_LIBRARY_PATH="${APPDIR}/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export GSK_RENDERER="${GSK_RENDERER:-cairo}"
export MALLOC_ARENA_MAX="${MALLOC_ARENA_MAX:-2}"
exec "${APPDIR}/usr/bin/umi" "$@"
EOF
chmod 0755 "$APPDIR/AppRun"

APPIMAGETOOL="${APPIMAGETOOL:-$(command -v appimagetool || command -v appimagetool-x86_64.AppImage || true)}"
if [[ -z "$APPIMAGETOOL" ]]; then
  echo "AppDir prepared at $APPDIR"
  echo "Install appimagetool to package into $OUT_APPIMAGE."
  exit 1
fi

UPDATE_INFO="gh-releases-zsync|YuukiWhite|UMI|latest|UMI-*x86_64.AppImage.zsync"
mkdir -p "$DIST_DIR"
(
  cd "$DIST_DIR"
  ARCH=x86_64 "$APPIMAGETOOL" \
    --appimage-extract-and-run \
    -u "$UPDATE_INFO" \
    "$APPDIR" \
    "$OUT_APPIMAGE" 2>/dev/null \
  || ARCH=x86_64 "$APPIMAGETOOL" -u "$UPDATE_INFO" "$APPDIR" "$OUT_APPIMAGE"
)

rm -rf "$APPDIR"
echo "Built $OUT_APPIMAGE"
ls -lh "$DIST_DIR"/UMI-"${VERSION}"-x86_64.AppImage*
