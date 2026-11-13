#!/bin/bash
# Build a distributable disk image: dist/Flowline-<version>.dmg
#
#   scripts/package.sh                  # universal build, ad-hoc signed
#   UNIVERSAL=0 scripts/package.sh      # host architecture only (quick local test)
#
# Environment (all optional):
#   CODESIGN_IDENTITY  "Developer ID Application: …"  signs the app and the DMG
#   NOTARY_PROFILE     a `xcrun notarytool store-credentials` profile; submits
#                      the DMG to Apple and staples the ticket
# Without them the DMG still works, but macOS shows a Gatekeeper warning on other
# Macs the first time it is opened (see README → Installing).
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
DMG="dist/Flowline-${VERSION}.dmg"

if [ "${UNIVERSAL:-1}" = 1 ]; then
  scripts/bundle.sh --universal
else
  scripts/bundle.sh
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -R dist/Flowline.app "$STAGE/Flowline.app"
ln -s /Applications "$STAGE/Applications"

rm -f "$DMG"
hdiutil create -volname "Flowline" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null

if [ -n "${CODESIGN_IDENTITY:-}" ]; then
  codesign --force --timestamp --sign "$CODESIGN_IDENTITY" "$DMG"
fi
if [ -n "${NOTARY_PROFILE:-}" ]; then
  xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait
  xcrun stapler staple "$DMG"
fi

shasum -a 256 "$DMG" | tee "$DMG.sha256"
echo "packaged $DMG"
