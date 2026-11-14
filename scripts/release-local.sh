#!/usr/bin/env bash
# Local release of the macOS side, on the maintainer's Mac (arm64): no hosted runner, no CI cost.
#
#   bash scripts/release-local.sh <tag> [--dry-run]
#
# <tag> is v<version> (vX.Y.Z or vX.Y.Z-rc.N). The script
#   1. checks before any build: `version.sh check <tag>`, a clean working tree, HEAD == main ==
#      origin/main (after `git fetch origin main`), no ROM data in the repository
#      (check-no-rom-data.sh), the release notes (the CHANGELOG.md section of the base version,
#      scripts/release/changelog-section.sh), the updater key (a real release needs it, see
#      below); for a real release also `gh auth status`, that no release of the tag exists yet
#      and that a tag of that name (local or on origin) is HEAD. Besides these checks it says
#      where the last green full run is (the step "Volllauf", right after the main check): see
#      below, a warning that never stops the release;
#   2. builds like the former bundle-macos job of release.yml: `pnpm install --frozen-lockfile`,
#      the tools from prepare-bundle.sh (apicula and rotom-lsp at the versions of
#      scripts/tool-versions.env, built into tools/ when missing; cargo-about must be installed;
#      its own step, so what it builds from Git never has the updater key in its environment),
#      `CI=true G4_BUNDLE_SKIP_PREPARE=1 pnpm bundle -- --bundles app,dmg` (the preparation is
#      done: only `tauri build` gets the key; CI=true: Tauri's DMG script
#      skips the Finder styling by AppleScript), then scripts/release/macos-dmg.sh (Tauri's DMG or
#      the hdiutil fallback, named Gen4-Studio_<version>_aarch64.dmg). With the updater key the
#      bundle also gets `--config src-tauri/tauri.updater.conf.json`: Tauri then signs the update
#      bundle `Gen4 Studio.app.tar.gz` and writes its `.sig` (the updater of the packed app
#      installs only packages with a valid signature);
#   3. checks the result: the update bundle (not empty, `Gen4 Studio.app/` first in the archive,
#      signature not empty), check-bundle.sh, check-no-rom-data.sh <app> <dmg>, and the smoke test
#      (smoke-packaged.sh, clean environment) on a synthetic project from make_synthetic;
#   4. writes dist-release/: the DMG, with the key Gen4-Studio_<version>_aarch64.app.tar.gz and
#      its .sig, SHA256SUMS.txt (shasum -a 256, only the DMG: the installer's line comes from
#      release.yml) and notes.md;
#   5. without --dry-run: `git tag -a <tag>` (when missing), `git push origin <tag>` and
#      `gh release create <tag> --draft [--prerelease]` with the DMG, the update bundle and its
#      .sig and SHA256SUMS.txt. The Windows installer follows from release.yml
#      (workflow_dispatch, see docs/release.md), which uploads it and its .sig to this draft, adds
#      its line to SHA256SUMS.txt and writes latest.json (scripts/release/updater-manifest.sh).
# --dry-run does all of 1 to 4 (without the gh and tag checks) and ends with
# `release-local: Probelauf ok (<files>)`. Nothing is tagged, pushed or published.
#
# The last green full run (the loose release rule: the maintainer decides, nothing blocks): a full
# run of `scripts/gate.sh --full` leaves <full sha>.json (field `ergebnis`: gruen | rot |
# unvollstaendig) in G4_GATE_ARTEFAKTE (default: ../gate-artefakte/full beside the main checkout;
# scripts/gate-result.sh). Along `git rev-list HEAD` (200 commits at most) the first commit whose
# result says `gruen` is looked for; a result that is rot or unvollstaendig, a missing or an
# unreadable file does not count. Exactly one of these comes out, the release always goes on:
#   Volllauf: grün auf diesem Commit (<sha>)
#   WARNUNG Volllauf: letzter grüner Stand <sha> (<n> Commits zurück)
#     followed by `git log --oneline <sha>..HEAD` and `git diff --stat <sha>..HEAD`
#   WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren
# (<sha>: 12 characters.) The release is then made on a commit whose ROM matrix did not run, if the
# maintainer chooses so: the price of the loose rule. Nothing here changes the exit status, not
# even a failing git or python3.
#
# The updater key (spec 1.0.1, 3.2 and 3.5; docs/release.md, "Update-Pakete"): the private key is
# TAURI_SIGNING_PRIVATE_KEY (its content) or, when that is empty, the content of the file
# G4_UPDATER_KEY; its password is TAURI_SIGNING_PRIVATE_KEY_PASSWORD (set, also an empty one) or,
# when that is not set, the keychain entry (service gen4-studio-updater, account $USER; macOS
# `security`). A real release needs both and stops before the build without them. A dry run
# without them builds without update packages and says so. Key and password are never printed.
# They are kept in shell variables that are not exported and reach only the bundle command
# (`pnpm bundle`, where Tauri reads them), so `pnpm install`, the preparation of the tools
# (prepare-bundle.sh: `cargo install` from Git, `cargo build`), the smoke test, `git` and `gh`
# never see them. Variables the caller exported already are un-exported (`export -n`) after the
# key step for the same reason: the caller's own shell keeps them, the children of this script do
# not.
#
# Variables (all optional):
#   G4_RELEASE_ROOT          the repository to release (default: this script's repository; tests)
#   G4_RELEASE_ALLOW_BRANCH  1: skip the HEAD == main == origin/main check, only with --dry-run
#                            (a dry run from a feature branch or a worktree)
#   G4_RELEASE_SKIP_BUILD    1: tests only, needs G4_RELEASE_ROOT. No build and no checks of the
#                            bundle: placeholder files stand for the DMG (and the update bundle
#                            and its .sig), so steps 4 and 5 can be tested.
#   G4_GATE_ARTEFAKTE        the folder of the results of full runs (see above; tests)
#   G4_UPDATER_KEY           the private updater key file (default ~/.tauri/gen4-studio-updater.key)
#   G4_UPDATER_KEYCHAIN      0: do not ask the keychain for the password (tests)
#
# Signing and notarization are prepared, not enabled (spec release-ci 5): the DMG is signed ad hoc
# (`bundle.macOS.signingIdentity: "-"` in apps/studio/src-tauri/tauri.macos.conf.json). With a
# Developer ID it would work like this on the Mac:
#   - the "Developer ID Application" certificate in the login keychain; signingIdentity in
#     tauri.macos.conf.json set to its name (or APPLE_SIGNING_IDENTITY exported before the bundle
#     step), hardenedRuntime back on (see docs/release.md, "Signatur");
#   - for notarization APPLE_ID, APPLE_PASSWORD (an app-specific password) and APPLE_TEAM_ID in
#     the environment of the bundle step: tauri-bundler signs the .app and runs notarytool itself;
#   - after the DMG step the DMG itself (see sign_notarize below):
#       xcrun notarytool submit "$DMG" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" \
#         --team-id "$APPLE_TEAM_ID" --wait
#       xcrun stapler staple "$DMG"
#   (APPLE_CERTIFICATE and APPLE_CERTIFICATE_PASSWORD, the base64 .p12 of the former workflow, are
#   only needed where the certificate is not in a keychain.)
#
# Works with bash 3.2 (macOS). Messages are German, like the other release scripts.
set -euo pipefail

SCRIPTS="$(cd "$(dirname "$0")" && pwd)"
ROOT="${G4_RELEASE_ROOT:-$(cd "$SCRIPTS/.." && pwd)}"
ROOT="$(cd "$ROOT" && pwd)"
# gate_main_checkout, gate_artefakte_dir and gate_result_of: where the results of full runs are and how
# one is read, written once for the gate and the nightly run (functions only).
# shellcheck source=gate-result.sh
. "$SCRIPTS/gate-result.sh"

die() {
  echo "release-local: $*" >&2
  exit 1
}

usage() {
  echo "usage: release-local.sh <tag> [--dry-run]" >&2
  exit 2
}

step() { # step <text>: a header line per step
  echo
  echo "==> release-local: $*"
}

# full_run_note <head sha>: says where the last green full run is (see the header). Only a note: no
# command in it may end the script, so each one that can fail has its own `|| true` (the caller adds
# one more for the function). Nothing is written, the folder of the results is not made.
full_run_note() {
  local dir sha green="" count
  dir="$(gate_artefakte_dir "$(gate_main_checkout "$ROOT")" "$ROOT")" || true
  # An unreadable history gives an empty list, which is "no green result" as well.
  for sha in $(git rev-list --max-count=200 HEAD 2>/dev/null || true); do
    if [ "$(gate_result_of "$dir/$sha.json" || true)" = "gruen" ]; then
      green="$sha"
      break
    fi
  done
  if [ -z "$green" ]; then
    echo "WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren"
  elif [ "$green" = "$1" ]; then
    echo "Volllauf: grün auf diesem Commit (${green:0:12})"
  else
    count="$(git rev-list --count "$green..HEAD" 2>/dev/null || true)"
    echo "WARNUNG Volllauf: letzter grüner Stand ${green:0:12} (${count:-?} Commits zurück)"
    git --no-pager log --oneline "$green..HEAD" || true
    git --no-pager diff --stat "$green..HEAD" || true
  fi
  return 0
}

TAG=""
DRY_RUN=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    -*) usage ;;
    *)
      [ -z "$TAG" ] || usage
      TAG="$arg"
      ;;
  esac
done
[ -n "$TAG" ] || usage
case "$TAG" in
  v[0-9]*) ;;
  *) die "Tag $TAG: erwartet v<Fassung>, etwa v0.1.0 oder v0.1.0-rc.1" ;;
esac
if [ "${G4_RELEASE_ALLOW_BRANCH:-}" = "1" ] && [ "$DRY_RUN" -ne 1 ]; then
  die "G4_RELEASE_ALLOW_BRANCH=1 gilt nur mit --dry-run: ein Release entsteht nur von main"
fi
SKIP_BUILD=0
if [ "${G4_RELEASE_SKIP_BUILD:-}" = "1" ]; then
  # Never a placeholder DMG in a real release: only for a test repository.
  [ -n "${G4_RELEASE_ROOT:-}" ] || die "G4_RELEASE_SKIP_BUILD=1 gilt nur mit G4_RELEASE_ROOT (Tests)"
  SKIP_BUILD=1
fi

# Use the pinned toolchain when it exists, otherwise whatever the shell has (like gate.sh).
TOOLCHAIN_BIN="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
if [ -d "$TOOLCHAIN_BIN" ]; then
  export PATH="$TOOLCHAIN_BIN:$PATH"
fi
# cargo-about (prepare-bundle.sh) lives there after `cargo install`; appended, shadows nothing.
if [ -d "$HOME/.cargo/bin" ]; then
  export PATH="$PATH:$HOME/.cargo/bin"
fi

cd "$ROOT"
VERSION="${TAG#v}"
case "$VERSION" in
  *-*) PRERELEASE=1 ;;
  *) PRERELEASE=0 ;;
esac
OUT="$ROOT/dist-release"
DMG_NAME="Gen4-Studio_${VERSION}_aarch64.dmg"
APP_TGZ_NAME="Gen4-Studio_${VERSION}_aarch64.app.tar.gz"

# --- 1. checks before the build ------------------------------------------------------------------

step "Fassung"
G4_VERSION_ROOT="$ROOT" bash "$SCRIPTS/version.sh" check "$TAG" || die "Fassung passt nicht zum Tag $TAG (siehe oben)"

step "Arbeitsbaum"
git rev-parse --git-dir >/dev/null 2>&1 || die "$ROOT ist kein Git-Repository"
if [ -n "$(git status --porcelain --untracked-files=normal)" ]; then
  git status --short >&2
  die "der Arbeitsbaum ist nicht sauber: erst committen oder aufräumen"
fi
echo "sauber"

HEAD_SHA="$(git rev-parse HEAD)"
if [ "${G4_RELEASE_ALLOW_BRANCH:-}" = "1" ]; then
  echo "G4_RELEASE_ALLOW_BRANCH=1: HEAD $HEAD_SHA wird nicht mit main verglichen (nur Probelauf)"
else
  step "main"
  git fetch --quiet origin main || die "git fetch origin main ist gescheitert"
  MAIN_SHA="$(git rev-parse --verify --quiet refs/heads/main)" || die "es gibt keinen lokalen Branch main"
  ORIGIN_SHA="$(git rev-parse --verify --quiet refs/remotes/origin/main)" || die "es gibt kein origin/main"
  [ "$HEAD_SHA" = "$MAIN_SHA" ] || die "HEAD ($HEAD_SHA) ist nicht main ($MAIN_SHA): ein Release entsteht nur von main"
  [ "$MAIN_SHA" = "$ORIGIN_SHA" ] || die "main ($MAIN_SHA) ist nicht origin/main ($ORIGIN_SHA): erst pushen oder pullen"
  echo "HEAD = main = origin/main ($HEAD_SHA)"
fi

step "Volllauf"
full_run_note "$HEAD_SHA" || true

step "keine ROM-Daten im Repository"
bash "$SCRIPTS/check-no-rom-data.sh" || die "ROM-Daten im Repository (siehe oben)"

step "Release-Notizen"
rm -rf "$OUT"
mkdir -p "$OUT"
bash "$SCRIPTS/release/changelog-section.sh" "$VERSION" "$ROOT/CHANGELOG.md" >"$OUT/notes.md" ||
  die "keine Release-Notizen für $VERSION (siehe oben)"
echo "$(wc -l <"$OUT/notes.md" | tr -d ' ') Zeilen aus CHANGELOG.md"

step "Updater-Schlüssel"
# Key and password stay in plain shell variables (not exported, see the header): the bundle step
# passes them on to Tauri, nothing else sees them. They are never printed, only where they come from.
UPDATER_KEY_FILE="${G4_UPDATER_KEY:-$HOME/.tauri/gen4-studio-updater.key}"
SIGN=0
SIGN_MISSING=""
KEY_FROM=""
PW_FROM=""
if [ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  KEY_FROM="Umgebung"
elif [ -f "$UPDATER_KEY_FILE" ] && [ -r "$UPDATER_KEY_FILE" ]; then
  TAURI_SIGNING_PRIVATE_KEY="$(cat "$UPDATER_KEY_FILE")"
  [ -z "$TAURI_SIGNING_PRIVATE_KEY" ] || KEY_FROM="Datei $UPDATER_KEY_FILE"
fi
if [ -z "$KEY_FROM" ]; then
  SIGN_MISSING="weder TAURI_SIGNING_PRIVATE_KEY noch die Datei $UPDATER_KEY_FILE (G4_UPDATER_KEY)"
else
  if [ -n "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD+x}" ]; then
    PW_FROM="Umgebung"
  elif [ "$(uname -s)" = "Darwin" ] && [ "${G4_UPDATER_KEYCHAIN:-}" != "0" ] && command -v security >/dev/null 2>&1; then
    if KEYCHAIN_PW="$(security find-generic-password -s gen4-studio-updater -a "${USER:-$(id -un)}" -w 2>/dev/null)"; then
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$KEYCHAIN_PW"
      PW_FROM="Schlüsselbund"
    fi
    KEYCHAIN_PW=""
  fi
  if [ -z "$PW_FROM" ]; then
    SIGN_MISSING="kein Passwort: weder TAURI_SIGNING_PRIVATE_KEY_PASSWORD noch der Schlüsselbund-Eintrag gen4-studio-updater (Konto ${USER:-$(id -un)})"
  fi
fi
# Whatever the caller exported: from here on only the bundle step gets the key and the password.
export -n TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD
if [ -z "$SIGN_MISSING" ]; then
  SIGN=1
  echo "Updater-Pakete: werden signiert (Schlüssel: $KEY_FROM, Passwort: $PW_FROM)"
elif [ "$DRY_RUN" -eq 1 ]; then
  echo "$SIGN_MISSING"
  echo "Updater-Pakete: ausgelassen (kein Schlüssel)"
else
  die "Updater-Schlüssel fehlt: $SIGN_MISSING (docs/release.md, Abschnitt „Update-Pakete“)"
fi

if [ "$DRY_RUN" -ne 1 ]; then
  step "GitHub und Tag"
  command -v gh >/dev/null 2>&1 || die "gh fehlt (brew install gh)"
  gh auth status >/dev/null 2>&1 || die "gh ist nicht angemeldet (gh auth login)"
  if gh release view "$TAG" >/dev/null 2>&1; then
    die "zum Tag $TAG gibt es schon einen Release (auch ein Entwurf zählt): erst auf GitHub löschen"
  fi
  if LOCAL_TAG_SHA="$(git rev-parse --verify --quiet "refs/tags/$TAG^{commit}")"; then
    [ "$LOCAL_TAG_SHA" = "$HEAD_SHA" ] || die "der lokale Tag $TAG zeigt auf $LOCAL_TAG_SHA, nicht auf HEAD ($HEAD_SHA)"
  fi
  REMOTE_TAG_SHA="$(git ls-remote origin "refs/tags/$TAG^{}" | cut -f1)" ||
    die "git ls-remote origin ist gescheitert"
  if [ -z "$REMOTE_TAG_SHA" ]; then
    REMOTE_TAG_SHA="$(git ls-remote origin "refs/tags/$TAG" | cut -f1)"
  fi
  if [ -n "$REMOTE_TAG_SHA" ] && [ "$REMOTE_TAG_SHA" != "$HEAD_SHA" ]; then
    die "der Tag $TAG auf origin zeigt auf $REMOTE_TAG_SHA, nicht auf HEAD ($HEAD_SHA)"
  fi
  echo "gh angemeldet, kein Release zu $TAG, Tag frei oder auf HEAD"
fi

# --- 2. and 3. build and checks ------------------------------------------------------------------

# The DMG still unsigned for a Developer ID: see the header. Enabling it means replacing this
# function body by the notarytool and stapler lines there (and the bundle environment).
sign_notarize() { # sign_notarize <dmg>
  echo "Signieren und Notarisieren: nicht aktiviert (siehe Kopf von release-local.sh)"
}

if [ "$SKIP_BUILD" -eq 1 ]; then
  step "Bau ausgelassen (G4_RELEASE_SKIP_BUILD=1, nur für Tests)"
  echo "placeholder for $DMG_NAME" >"$OUT/$DMG_NAME"
  if [ "$SIGN" -eq 1 ]; then
    echo "placeholder for $APP_TGZ_NAME" >"$OUT/$APP_TGZ_NAME"
    echo "placeholder for $APP_TGZ_NAME.sig" >"$OUT/$APP_TGZ_NAME.sig"
  fi
else
  [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ] ||
    die "das DMG entsteht nur auf macOS arm64 (hier: $(uname -s) $(uname -m))"
  # shellcheck source=tool-versions.env
  . "$SCRIPTS/tool-versions.env"

  step "Werkzeuge"
  command -v cargo >/dev/null 2>&1 || die "cargo fehlt (siehe docs/dev-setup.md)"
  command -v pnpm >/dev/null 2>&1 || die "pnpm fehlt (siehe docs/dev-setup.md)"
  about="$(cargo about --version 2>/dev/null | awk '{print $2}' || true)"
  [ -n "$about" ] || die "cargo-about fehlt: cargo install cargo-about --version $CARGO_ABOUT_VERSION --locked"
  [ "$about" = "$CARGO_ABOUT_VERSION" ] ||
    die "cargo-about $about, erwartet $CARGO_ABOUT_VERSION (scripts/tool-versions.env): cargo install cargo-about --version $CARGO_ABOUT_VERSION --locked --force"
  pnpm_v="$(pnpm --version)"
  [ "$pnpm_v" = "$PNPM_VERSION" ] || echo "Warnung: pnpm $pnpm_v, tool-versions.env nennt $PNPM_VERSION" >&2
  node_major="$(node --version | sed -n 's/^v\([0-9]*\).*/\1/p')"
  [ "$node_major" = "$NODE_VERSION" ] || echo "Warnung: Node $(node --version), tool-versions.env nennt $NODE_VERSION" >&2
  echo "cargo-about $about, pnpm $pnpm_v, node $(node --version); apicula und rotom-lsp: prepare-bundle.sh"

  step "pnpm install"
  (cd apps/studio && pnpm install --frozen-lockfile)

  step "Werkzeuge bereitlegen"
  # Here, not inside the bundle command: the key is in no exported variable at this point, so
  # what prepare-bundle.sh builds from Git (rotom-lsp, apicula) and compiles never sees it. The
  # bundle step skips the preparation (G4_BUNDLE_SKIP_PREPARE=1) instead of running it again.
  bash apps/studio/scripts/prepare-bundle.sh

  step "Bundle (app, dmg)"
  BUNDLE="$ROOT/target/release/bundle"
  bundle_args=(--bundles app,dmg)
  if [ "$SIGN" -eq 1 ]; then
    # createUpdaterArtifacts: the .app.tar.gz and its .sig (Tauri reads the key from the environment
    # of the bundle command, see below).
    bundle_args+=(--config src-tauri/tauri.updater.conf.json)
  fi
  # A DMG or .app of an earlier build must never stand in for this one (macos-dmg.sh takes the
  # first DMG it finds).
  rm -rf "$BUNDLE/dmg" "$BUNDLE/macos"
  BUNDLE_STATUS=0
  (
    cd apps/studio
    if [ "$SIGN" -eq 1 ]; then
      # Exported in this subshell only: the one command that needs the key and the password.
      export TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD
    fi
    CI=true G4_BUNDLE_SKIP_PREPARE=1 pnpm bundle -- "${bundle_args[@]}"
  ) || BUNDLE_STATUS=$?
  echo "Bundle-Status: $BUNDLE_STATUS"

  step "DMG"
  # macos-dmg.sh reports the .app and the DMG through a GitHub output file. No CI=true here: it
  # would detach a mounted "Gen4 Studio" volume, which on this Mac may be the user's.
  DMG_INFO="$(mktemp "${TMPDIR:-/tmp}/release-local-dmg.XXXXXX")"
  BUNDLE_STATUS="$BUNDLE_STATUS" GITHUB_OUTPUT="$DMG_INFO" GITHUB_STEP_SUMMARY="" \
    bash "$SCRIPTS/release/macos-dmg.sh" "$VERSION" "$OUT"
  APP="$(sed -n 's/^app=//p' "$DMG_INFO")"
  DMG="$(sed -n 's/^dmg=//p' "$DMG_INFO")"
  rm -f "$DMG_INFO"
  [ -d "$APP" ] && [ -s "$DMG" ] || die "macos-dmg.sh hat kein .app oder DMG gemeldet"
  [ "$DMG" = "$OUT/$DMG_NAME" ] || die "DMG $DMG, erwartet $OUT/$DMG_NAME"

  if [ "$SIGN" -eq 1 ]; then
    step "Update-Paket"
    UPDATE_TGZ="$BUNDLE/macos/Gen4 Studio.app.tar.gz"
    [ -s "$UPDATE_TGZ" ] ||
      die "Update-Paket fehlt oder ist leer: $UPDATE_TGZ (Bundle-Status $BUNDLE_STATUS; Schlüssel oder Passwort falsch, oder das Bundle selbst ist gescheitert, etwa das DMG: siehe die Ausgabe von „Bundle (app, dmg)“ oben)"
    [ -s "$UPDATE_TGZ.sig" ] || die "Signatur fehlt oder ist leer: $UPDATE_TGZ.sig (Bundle-Status $BUNDLE_STATUS)"
    FIRST_ENTRY="$(tar -tzf "$UPDATE_TGZ" | sed -n '1p')" || die "$UPDATE_TGZ ist kein lesbares tar.gz"
    [ "$FIRST_ENTRY" = "Gen4 Studio.app/" ] ||
      die "erster Eintrag in $UPDATE_TGZ: '$FIRST_ENTRY', erwartet 'Gen4 Studio.app/'"
    cp "$UPDATE_TGZ" "$OUT/$APP_TGZ_NAME"
    cp "$UPDATE_TGZ.sig" "$OUT/$APP_TGZ_NAME.sig"
    echo "$APP_TGZ_NAME und $APP_TGZ_NAME.sig"
  fi

  step "Signieren und Notarisieren"
  sign_notarize "$DMG"

  step "check-bundle"
  bash "$ROOT/apps/studio/scripts/check-bundle.sh"

  step "keine ROM-Daten im Paket"
  bash "$SCRIPTS/check-no-rom-data.sh" "$APP" "$DMG"

  step "Rauchtest"
  SYN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/release-local-syn.XXXXXX")"
  trap 'rm -rf "$SYN_DIR"' EXIT
  # The synthetic project is built from code (test_support::synthetic_project): no ROM byte.
  cargo run -p g4-kernel --example make_synthetic --features test-support -- "$SYN_DIR/syn"
  bash "$SCRIPTS/smoke-packaged.sh" "$APP/Contents/MacOS/studio" "$SYN_DIR/syn"
fi

# --- 4. checksums --------------------------------------------------------------------------------

step "SHA256SUMS.txt"
(cd "$OUT" && shasum -a 256 "$DMG_NAME" >SHA256SUMS.txt)
cat "$OUT/SHA256SUMS.txt"

if [ "$SIGN" -eq 1 ]; then
  FILES="dist-release/$DMG_NAME, dist-release/$APP_TGZ_NAME, dist-release/$APP_TGZ_NAME.sig, dist-release/SHA256SUMS.txt, dist-release/notes.md"
else
  FILES="dist-release/$DMG_NAME, dist-release/SHA256SUMS.txt, dist-release/notes.md"
fi
if [ "$DRY_RUN" -eq 1 ]; then
  echo
  echo "release-local: Probelauf ok ($FILES)"
  exit 0
fi

# --- 5. tag, push, draft release -----------------------------------------------------------------

step "Tag $TAG"
if git rev-parse --verify --quiet "refs/tags/$TAG" >/dev/null; then
  echo "Tag $TAG gibt es schon (auf HEAD)"
else
  git tag -a "$TAG" -m "Gen4 Studio $VERSION"
fi
git push origin "refs/tags/$TAG" || die "git push origin $TAG ist gescheitert"

step "Release-Entwurf"
args=(--draft --verify-tag --title "Gen4 Studio $VERSION" --notes-file "$OUT/notes.md")
if [ "$PRERELEASE" -eq 1 ]; then
  args+=(--prerelease)
fi
gh release create "$TAG" "${args[@]}" "$OUT/$DMG_NAME" "$OUT/$APP_TGZ_NAME" "$OUT/$APP_TGZ_NAME.sig" "$OUT/SHA256SUMS.txt" ||
  die "gh release create ist gescheitert (Tag $TAG ist gepusht; den Schritt von Hand wiederholen)"

echo
echo "release-local: Entwurf $TAG angelegt ($FILES)"
echo "Windows-Installer, seine .sig und latest.json dazu: gh workflow run release.yml -f tag=$TAG -f dry_run=false"
echo "Danach hat der Entwurf sieben Dateien (docs/release.md, Abschnitt „Update-Pakete“)."
