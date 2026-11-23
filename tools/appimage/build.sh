#!/usr/bin/env bash
# Build dist/MobaScape-x86_64.AppImage: a launcher that updates, rebuilds and
# runs this checkout on every start (see tools/appimage/AppRun). It only needs
# rebuilding when the launcher itself changes or the checkout moves; game
# updates are picked up automatically.
#
# Contents: AppRun, desktop file, icon, the checkout path and a snapshot
# rift-client binary as a fallback. No cache, no assets/: Jagex data never
# goes into the AppImage.
#
# Usage: tools/appimage/build.sh [--install]
#   --install  also copy it to ~/Applications and add a menu entry.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="$ROOT/tools/appimage"
APPDIR="$ROOT/target/appimage/MobaScape.AppDir"
OUT="$ROOT/dist/MobaScape-x86_64.AppImage"
TOOL_DIR="$ROOT/.cache/tools"
TOOL="$TOOL_DIR/appimagetool-x86_64.AppImage"
TOOL_URL="https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"

echo "==> building rift-client (release)"
cargo build --release -p rift-client --manifest-path "$ROOT/Cargo.toml"

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

if [ ! -x "$TOOL" ]; then
    echo "==> fetching appimagetool"
    mkdir -p "$TOOL_DIR"
    curl -fL --retry 3 -o "$TOOL.part" "$TOOL_URL"
    chmod +x "$TOOL.part"
    mv "$TOOL.part" "$TOOL"
fi

echo "==> packing $OUT"
mkdir -p "$ROOT/dist"
# Extract-and-run avoids needing FUSE for the tool itself.
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" --no-appstream "$APPDIR" "$OUT"
chmod +x "$OUT"
echo "==> wrote $OUT ($(du -h "$OUT" | cut -f1))"

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
