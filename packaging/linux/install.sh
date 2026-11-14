#!/bin/sh
# Install Flowline for the current user, no root needed:
#
#   sh install.sh            # to ~/.local/opt/flowline, linked from ~/.local/bin
#   sh install.sh --remove   # take it out again
#
# It copies the files here, adds an applications-menu entry and an icon, and
# nothing else. Your settings and downloaded models are kept on --remove.
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"
OPT="$HOME/.local/opt/flowline"
BIN="$HOME/.local/bin"

if [ "${1:-}" = "--remove" ]; then
  rm -rf "$OPT"
  rm -f "$BIN/flowline" "$PREFIX/applications/flowline.desktop" "$PREFIX/icons/hicolor/512x512/apps/flowline.png"
  echo "Flowline removed. Your data in $PREFIX/flowline was kept."
  exit 0
fi

mkdir -p "$OPT" "$BIN" "$PREFIX/applications" "$PREFIX/icons/hicolor/512x512/apps"
cp -R "$HERE/flowline" "$HERE/bin" "$HERE/flowline-setup.sh" "$HERE/README.txt" "$HERE/LICENSE" "$HERE/THIRD-PARTY-NOTICES.md" "$OPT/" 2>/dev/null || {
  # `bin` is absent when the archive has no bundled engine.
  cp -R "$HERE/flowline" "$HERE/flowline-setup.sh" "$HERE/README.txt" "$HERE/LICENSE" "$HERE/THIRD-PARTY-NOTICES.md" "$OPT/"
}
chmod +x "$OPT/flowline" "$OPT/flowline-setup.sh"
ln -sf "$OPT/flowline" "$BIN/flowline"
cp "$HERE/flowline.png" "$PREFIX/icons/hicolor/512x512/apps/flowline.png"
sed "s|^Exec=.*|Exec=$OPT/flowline|" "$HERE/flowline.desktop" > "$PREFIX/applications/flowline.desktop"

echo "Installed to $OPT"
echo "Next: run   sh $OPT/flowline-setup.sh   once (keyboard and paste access), then log out and in."
case ":$PATH:" in *":$BIN:"*) ;; *) echo "Note: $BIN is not on your PATH; start Flowline from the applications menu." ;; esac
