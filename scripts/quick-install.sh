#!/usr/bin/env bash
set -euo pipefail

REPO="YuukiWhite/UMI"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"

echo "Fetching latest UMI release from GitHub..."
TAR_URL="$(curl -fsSL "$API_URL" | grep -oE 'https://[^"]+x86_64-linux\.tar\.gz' | head -n 1)"

if [[ -z "${TAR_URL:-}" ]]; then
  echo "Error: Could not find a release .tar.gz asset at ${API_URL}."
  echo "Create a release tag first (e.g. git tag v1.3.1 && git push --tags)."
  exit 1
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading ${TAR_URL}..."
curl -fL --progress-bar "$TAR_URL" -o "$TMP_DIR/umi.tar.gz"
tar -xzf "$TMP_DIR/umi.tar.gz" -C "$TMP_DIR"

EXTRACTED_DIR="$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [[ -z "${EXTRACTED_DIR:-}" || ! -x "$EXTRACTED_DIR/install.sh" ]]; then
  echo "Error: Unexpected archive layout."
  exit 1
fi

pkill -x umi >/dev/null 2>&1 || true
"$EXTRACTED_DIR/install.sh"
echo "Done! Launch 'UMI' from your app launcher or run: umi"
