#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-$(awk -F'"' '/^version =/{print $2; exit}' "$ROOT_DIR/Cargo.toml")}"
DIST_DIR="$ROOT_DIR/dist"
RPMBUILD_DIR="$DIST_DIR/rpmbuild"

rm -rf "$RPMBUILD_DIR"
mkdir -p "$RPMBUILD_DIR"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# Create clean source tarball matching umi.spec Source0
tar --exclude='./target' \
    --exclude='./dist' \
    --exclude='./.git' \
    --transform "s,^\.,umi-${VERSION}," \
    -czf "$RPMBUILD_DIR/SOURCES/umi-${VERSION}.tar.gz" \
    -C "$ROOT_DIR" .

sed "s/^Version:.*/Version:        ${VERSION}/" "$ROOT_DIR/packaging/umi.spec" > "$RPMBUILD_DIR/SPECS/umi.spec"

rpmbuild --define "_topdir $RPMBUILD_DIR" -bb "$RPMBUILD_DIR/SPECS/umi.spec"

mkdir -p "$DIST_DIR"
cp "$RPMBUILD_DIR"/RPMS/x86_64/umi-*.rpm "$DIST_DIR/"
rm -rf "$RPMBUILD_DIR"
echo "Built RPM in $DIST_DIR:"
ls -lh "$DIST_DIR"/umi-*.rpm
