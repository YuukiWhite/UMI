#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ $# -lt 1 ]]; then
  echo "Usage: ./scripts/release.sh <version> [commit message]"
  echo "Example: ./scripts/release.sh 1.3.2 \"fix: smoother glass animation\""
  exit 1
fi

VERSION="${1#v}"
TAG="v${VERSION}"
MSG="${2:-release: UMI ${TAG}}"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: version must be semver (e.g. 1.3.2), got: $VERSION"
  exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Error: tag ${TAG} already exists locally. Pick a newer version."
  exit 1
fi

echo "==> Bumping version to ${VERSION} in Cargo.toml & packaging/umi.spec..."
sed -i -E "0,/^version = \".*\"/s//version = \"${VERSION}\"/" Cargo.toml
sed -i -E "s/^Version:.*/Version:        ${VERSION}/" packaging/umi.spec

echo "==> Running unit tests & syncing Cargo.lock..."
cargo test --offline || cargo test

echo "==> Staging & committing changes..."
git add -A
if git diff --cached --quiet; then
  echo "No staged changes to commit; creating tag on current HEAD."
else
  git commit -m "$MSG"
fi

echo "==> Creating tag ${TAG} & pushing to GitHub..."
git tag -a "$TAG" -m "UMI ${TAG}"
git push origin main "$TAG"

echo ""
echo "Done! GitHub Actions is now building ${TAG} (.AppImage + zsync, .rpm, .deb, .tar.gz):"
echo "  https://github.com/YuukiWhite/UMI/actions"
echo "Once finished (~4-5 min), Gear Lever will detect the update automatically."
