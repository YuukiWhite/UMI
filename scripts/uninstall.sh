#!/usr/bin/env bash
set -euo pipefail

rm -f "${HOME}/.local/bin/umi"
rm -f "${HOME}/.local/share/applications/org.umi.Notch.desktop"
rm -f "${HOME}/.local/share/icons/hicolor/scalable/apps/org.umi.Notch.svg"
rm -f "${HOME}/.config/autostart/umi.desktop"

if [[ "${1:-}" == "--purge" ]]; then
  rm -rf "${HOME}/.config/umi" "${HOME}/.local/share/umi"
  echo "Removed UMI binary, launcher, config, and local SQLite data."
else
  echo "Removed UMI binary & launcher. User config (~/.config/umi) and data (~/.local/share/umi) preserved."
  echo "Pass --purge to also remove config and local database."
fi
