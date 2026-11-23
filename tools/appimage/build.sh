#!/usr/bin/env bash
# Build dist/MobaScape-x86_64.AppImage: a launcher that updates, rebuilds and
# runs this checkout on every start (see tools/appimage/AppRun). It only needs
# rebuilding when the launcher itself changes or the checkout moves; game
# updates are picked up automatically.
#
# Contents: AppRun, desktop file, icon, the checkout path and a snapshot
# rift-client binary as a fallback. No cache, no assets/: Jagex data never
# goes into the AppImage. Packing uses crates/appimage (squashfs + runtime),
# so no external tool is downloaded.
#
# Usage: tools/appimage/build.sh [--install]
#   --install  also copy it to ~/Applications and add a menu entry.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="$ROOT/tools/appimage"
APPDIR="$ROOT/target/appimage/MobaScape.AppDir"
OUT="$ROOT/dist/MobaScape-x86_64.AppImage"

echo "==> building rift-client and the AppImage packer (release)"
cargo build --release -p rift-client -p rift-appimage --manifest-path "$ROOT/Cargo.toml"

echo "==> staging $APPDIR"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/scalable/apps"
install -m755 "$SRC/AppRun" "$APPDIR/AppRun"
install -m755 "$ROOT/target/release/rift-client" "$APPDIR/usr/bin/rift-client"
install -m644 "$SRC/mobascape.desktop" "$APPDIR/mobascape.desktop"
install -m644 "$SRC/mobascape.desktop" "$APPDIR/usr/share/applications/mobascape.desktop"
install -m644 "$SRC/mobascape.svg" "$APPDIR/mobascape.svg"
install -m644 "$SRC/mobascape.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/mobascape.svg"
ln -sf mobascape.svg "$APPDIR/.DirIcon"
printf '%s\n' "$ROOT" >"$APPDIR/repo-path"
git -C "$ROOT" log -1 --format='%H %s' >"$APPDIR/snapshot-commit" 2>/dev/null || true

echo "==> packing $OUT"
# In-repo packer and runtime (crates/appimage): no appimagetool download.
"$ROOT/target/release/rift-appimage" "$ROOT/target/release/rift-appimage-runtime" "$APPDIR" "$OUT"

if [ "${1:-}" = "--install" ]; then
    APPS="$HOME/Applications"
    mkdir -p "$APPS" "$HOME/.local/share/applications" "$HOME/.local/share/icons/hicolor/scalable/apps"
    cp -f "$OUT" "$APPS/MobaScape-x86_64.AppImage"
    cp -f "$SRC/mobascape.svg" "$HOME/.local/share/icons/hicolor/scalable/apps/mobascape.svg"
    sed "s|^Exec=.*|Exec=\"$APPS/MobaScape-x86_64.AppImage\" %U|" "$SRC/mobascape.desktop" \
        >"$HOME/.local/share/applications/mobascape.desktop"
    command -v update-desktop-database >/dev/null && update-desktop-database -q "$HOME/.local/share/applications" || true
    echo "==> installed $APPS/MobaScape-x86_64.AppImage and a MobaScape menu entry"
fi
