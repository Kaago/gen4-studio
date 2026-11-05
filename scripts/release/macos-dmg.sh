#!/usr/bin/env bash
# Picks the DMG of the macOS bundle, with the hdiutil fallback (spec release-ci 5), and names it
# for the release.
#
#   macos-dmg.sh <version> <out-dir>
#
# `pnpm bundle -- --bundles app,dmg` (CI=true, so Tauri's DMG script skips the Finder styling by
# AppleScript) normally leaves target/release/bundle/dmg/*.dmg. When it did not, or when the
# bundle step ended with an error (BUNDLE_STATUS not 0, so a DMG may be half written), the DMG is
# made from the finished .app by
#   hdiutil create -volname "Gen4 Studio" -srcfolder "<app>" -ov -format UDZO <dmg>
# Without a .app there is nothing to fall back on: the script fails. The result is
# <out-dir>/Gen4-Studio_<version>_aarch64.dmg; `dmg: tauri` or `dmg: hdiutil fallback` goes to the
# step summary, and `app`, `dmg` and `dmg_source` to $GITHUB_OUTPUT (both when set).
#
# Variables (all optional):
#   BUNDLE_STATUS          exit status of the bundle step (default 0)
#   G4_BUNDLE_DIR          the bundle folder (default: target/release/bundle of the repository)
#   G4_DMG_FORCE_FALLBACK  1: always take the hdiutil path (tests, local check)
#   G4_HDIUTIL             the hdiutil command (tests)
# Works with bash 3.2 (macOS).
set -euo pipefail

die() {
  echo "macos-dmg: $*" >&2
  exit 1
}

if [ $# -ne 2 ]; then
  echo "usage: macos-dmg.sh <version> <out-dir>" >&2
  exit 2
fi
VERSION="$1"
OUT="$2"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUNDLE="${G4_BUNDLE_DIR:-$ROOT/target/release/bundle}"
HDIUTIL="${G4_HDIUTIL:-hdiutil}"
STATUS="${BUNDLE_STATUS:-0}"

APP=""
for candidate in "$BUNDLE"/macos/*.app; do
  if [ -d "$candidate" ]; then
    APP="$candidate"
    break
  fi
done
if [ -z "$APP" ]; then
  die "kein .app in $BUNDLE/macos (Bundle-Status $STATUS): der DMG-Rueckfall braucht die fertige App"
fi

# A failed DMG script leaves its read-write image (rw.<pid>.<name>.dmg) behind: never take it.
rm -f "$BUNDLE"/dmg/rw.*.dmg 2>/dev/null || true
TAURI_DMG=""
for candidate in "$BUNDLE"/dmg/*.dmg; do
  if [ -s "$candidate" ]; then
    TAURI_DMG="$candidate"
    break
  fi
done

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
DMG="$OUT/Gen4-Studio_${VERSION}_aarch64.dmg"
rm -f "$DMG"

if [ -n "$TAURI_DMG" ] && [ "$STATUS" = "0" ] && [ "${G4_DMG_FORCE_FALLBACK:-}" != "1" ]; then
  cp "$TAURI_DMG" "$DMG"
  SOURCE="tauri"
else
  if [ "$STATUS" != "0" ]; then
    echo "::warning::bundle step ended with status $STATUS, DMG from hdiutil"
  fi
  # A volume that a failed Tauri DMG script left mounted (CI only: locally it may be the user's).
  if [ -n "${CI:-}" ] && [ -z "${G4_HDIUTIL:-}" ]; then
    for vol in "/Volumes/Gen4 Studio"*; do
      [ -d "$vol" ] && "$HDIUTIL" detach -force "$vol" >/dev/null 2>&1 || true
    done
  fi
  "$HDIUTIL" create -volname "Gen4 Studio" -srcfolder "$APP" -ov -format UDZO "$DMG"
  SOURCE="hdiutil fallback"
fi
[ -s "$DMG" ] || die "$DMG fehlt oder ist leer"

echo "dmg: $SOURCE ($DMG)"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  echo "dmg: $SOURCE" >>"$GITHUB_STEP_SUMMARY"
fi
if [ -n "${GITHUB_OUTPUT:-}" ]; then
  {
    echo "app=$APP"
    echo "dmg=$DMG"
    echo "dmg_source=$SOURCE"
  } >>"$GITHUB_OUTPUT"
fi
