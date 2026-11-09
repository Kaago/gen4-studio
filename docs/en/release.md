*Deutsch: [docs/release.md](../release.md)*

# Release: macOS DMG and Windows installer

Gen4 Studio ships as a DMG for macOS (arm64) and as an NSIS installer for Windows (x64). Both packages are built from the
hermetic sources: no ROM, no save game, no secret. `scripts/release-local.sh` builds the DMG on the maintainer's Mac, which also
creates the tag and the draft release; only the installer is built by the `release.yml` workflow on a hosted Windows runner,
started by hand. The CI workflow `ci.yml` runs on every pull request and every push to `main` (section "Workflows, tags and
versions"). This document describes both bundles, the way from tag to release, the licence notices and the prepared signing step.
The acceptance with a ROM and with the finished package is in `docs/release-ci-gate.md` (German).

The bundle contains third-party components so that the app runs without the development tree:

- nitroarc (macOS `Contents/Frameworks/libnitroarc_ffi.dylib`, Windows `nitroarc_ffi.dll` next to `studio.exe`): the
  LGPL-licensed library that uxie builds as a shared library during the release build. It stays a separate file
  (no static linking) so that it can be replaced (section "Replacing nitroarc").
- apicula (macOS `Contents/MacOS/apicula`, Windows `apicula.exe` next to `studio.exe`): the sidecar for NSBMD to GLB. When bundling, Tauri strips the
  target triple suffix (`apicula-aarch64-apple-darwin` becomes `apicula`). The app finds it through entry 2 of the
  apicula lookup (see `docs/en/dev-setup.md`); `G4_APICULA` does not have to be set.
- rotom-lsp (macOS `Contents/MacOS/rotom-lsp`, Windows `rotom-lsp.exe` next to `studio.exe`): the language server for scripts (Git tag
  `v0.7.0`, like the rotom library). On macOS it has the rpath `@executable_path/../Frameworks` and loads the same
  `libnitroarc_ffi.dylib`. The app finds it next to the Studio binary (entry 2 of the lookup in `docs/en/dev-setup.md`).

Play mode live (the melonDS core inside the window) stays macOS-only; on Windows the app says so with a notice.

## Prerequisites (local, macOS)

- apicula built at `tools/apicula/bin/apicula` (see `docs/en/dev-setup.md`).
- `rotom-lsp` at `tools/rotom/bin/rotom-lsp`; if it is missing, `prepare-bundle.sh` builds it from the Git tag (network needed the first time).
- No custom `PATH` needed any more: `pnpm bundle` and `pnpm tauri` run through `apps/studio/scripts/with-rust.sh`, which puts `/opt/homebrew/opt/rustup/bin` and `~/.cargo/bin` in front of the `PATH` (`tauri build` needs `cargo`; without it the build aborted with "failed to run 'cargo metadata'").
- The file `.cargo/config.toml` (CFLAGS for nitroarc on new macOS SDKs) is in the repository and applies automatically. On Windows it does no harm.
- The versions of the tools (apicula, rotom, cargo-deny, pnpm, Node, cargo-about) live in one place: `scripts/tool-versions.env`.

## macOS: the DMG

`pnpm bundle` builds only the `.app`; the DMG is made by `scripts/release-local.sh` (section "Workflows, tags and versions").

```sh
cd apps/studio
pnpm bundle                      # prepare-bundle.sh, then tauri build (the .app only)
bash scripts/check-bundle.sh     # checks the bundle, output: bundle ok
```

In detail, `pnpm bundle` does this:

1. `scripts/prepare-bundle.sh` runs `cargo build --release -p studio` (which produces `target/release/libnitroarc_ffi.dylib`)
   and copies apicula to `src-tauri/binaries/apicula-<target triple>`, `rotom-lsp` to
   `src-tauri/binaries/rotom-lsp-<target triple>` (with rpath, re-signed) and the dylib to
   `src-tauri/frameworks/libnitroarc_ffi.dylib`. It also generates the licence notices (section "Licence notices").
   If a source is missing, the script aborts with an error message. The target folders are excluded by `.gitignore`.
2. `tauri build` puts `externalBin` in `Contents/MacOS/`, `macOS.frameworks` in `Contents/Frameworks/` and the
   licence notices in `Contents/Resources/licenses/`. The result is `target/release/bundle/macos/Gen4 Studio.app`.

The DMG is made by `scripts/release-local.sh` with `pnpm bundle -- --bundles app,dmg` and `CI=true`: Tauri's DMG script then skips
styling the Finder window through AppleScript (on the runner this step was built for, it failed without a desktop session;
locally the result stays the same). This is the decision of the specification (Tauri instead of a script of our own). As a fallback
there is `scripts/release/macos-dmg.sh`: if Tauri did not deliver a DMG, the script builds it from the finished
`.app` with `hdiutil create -volname "Gen4 Studio" -srcfolder … -format UDZO` and reports `dmg: tauri` or `dmg: hdiutil fallback`.
Without an `.app` it fails. The file is named `Gen4-Studio_<version>_aarch64.dmg`.

**Signature:** The bundle as a whole is signed ad hoc (`bundle.macOS.signingIdentity: "-"` in `tauri.macos.conf.json`:
Tauri signs sidecars, framework and app with a bound `Info.plist` and sealed resources), not with a Developer ID
and not notarized. `check-bundle.sh` verifies this with `codesign --verify --deep --strict`. In addition, `hardenedRuntime: false`
is set: with Hardened Runtime, library validation rejects the ad-hoc signed `libnitroarc_ffi.dylib` ("different Team
IDs") and `rotom-lsp` does not start. A later notarization (Developer ID) needs Hardened Runtime again, then with
the entitlement `com.apple.security.cs.disable-library-validation` or a dylib signed with the same ID. Up to `0.1.0-rc.1` only the
binary carried the linker signature; a downloaded DMG then counted as "damaged" instead of "not verified" (gate G5, finding B1).
On first launch Gatekeeper therefore asks for approval:

- System Settings, Privacy & Security: after the first blocked start, find the message about Gen4 Studio and choose
  "Open Anyway".
- On macOS 14 and earlier you can also right-click the app, choose Open (and choose Open once more in the dialog).
- Or in the terminal: `xattr -dr com.apple.quarantine "/Applications/Gen4 Studio.app"`.

Only the architecture of the build machine (`aarch64-apple-darwin`); a universal binary is not planned. Minimum system:
macOS 13.0.

## Windows: the installer

The target is `x86_64-pc-windows-msvc`. This is the default of `windows-latest`: no `rustup target add`, no
`CARGO_BUILD_TARGET`, and `target/` is `target/debug` as on the Mac. The trial run on the runner compared both targets:
with MSVC, `studio` builds cold in 657 s (`cargo build -p studio`); with `x86_64-pc-windows-gnu` the link
of `studio` fails (`ld returned 8`). The GNU line was therefore dropped.

**nitroarc on Windows:** uxie builds nitroarc with `make ffi` and `gcc` into `nitroarc_ffi.dll`. This needs MinGW gcc and make
even with MSVC Rust; both come from MSYS2 (`msys2/setup-msys2`, package `mingw-w64-x86_64-gcc make`). The CFLAGS from
`.cargo/config.toml` (`-D_POSIX_C_SOURCE=200809L`) do no harm there; they stay for all platforms.

**Bundle:** NSIS (`pnpm bundle -- --bundles nsis`, under Git Bash). The settings are in
`apps/studio/src-tauri/tauri.windows.conf.json`:

- `installMode: currentUser`: the installer needs no administrator rights; the default folder is `%LOCALAPPDATA%\Gen4 Studio`
  (Tauri's NSIS template, `@tauri-apps/cli` 2.12.3: `StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"`; an earlier installation
  keeps its folder).
- WebView2 comes through the download bootstrapper (`downloadBootstrapper`), so the installer is small and needs the
  network on first install if WebView2 is missing.
- `nitroarc_ffi.dll` sits next to `studio.exe`. It is listed as a resource only in `apps/studio/src-tauri/tauri.bundle.windows.conf.json`,
  which `apps/studio/scripts/bundle.sh` passes to `tauri build` under Git Bash. Reason: on every
  `cargo build` and `cargo test`, `tauri-build` copies all `bundle.resources` next to the binary and would overwrite the real DLL (put there by uxie's
  `build.rs`) with the empty placeholder; `target\debug\studio.exe` would then no longer find the DLL.

**Signature:** The installer is not signed. SmartScreen warns on first start ("Windows protected your PC"):
"More info", then "Run anyway".

**Check:** `apps/studio/scripts/check-bundle-windows.sh <installer>` installs silently into a folder under `$RUNNER_TEMP`,
checks the files (`studio.exe`, `apicula.exe`, `rotom-lsp.exe`, the DLL, `licenses/`) and calls `studio.exe doctor`; the output ends with
`bundle ok (windows)`. After that `check-no-rom-data.sh` and the smoke test run over the installation folder.

### NSIS and pre-release versions

The specification (§6.4) asks whether NSIS accepts a version like `0.1.0-rc.1`; otherwise the Windows build
would get the file version `0.1.0.1` for `bundle.windows.nsis`, while the name of the artifact stays SemVer. The trial run
of the release workflow built the Windows side on pull request runs with `0.1.0-rc.1` for this (never checked in); a failing bundle would have been the answer.

Finding (trial run 41960501794, 2026-11-05): NSIS accepts `0.1.0-rc.1`. The bundle is named `Gen4 Studio_0.1.0-rc.1_x64-setup.exe`,
installs silently, and `studio doctor` reports `studio 0.1.0-rc.1`. A numeric substitute version (`0.1.0.1`) is not needed;
the workflow does not set one.

## Workflows, tags and versions

**CI on pull requests, no workflow on a tag.** As long as the repository was private (since 5 November 2026), no
push and no pull request started a workflow: hosted minutes cost money (a push to `main` ran at about $1.80, macOS minutes
count ten times). For public repositories the standard runners are free, so `ci.yml` now runs on every pull
request against `main` and on every push to `main`, without a path filter. The full `scripts/gate.sh` on the Mac before every merge
(`docs/en/dev-setup.md`) remains the acceptance; it also runs the ROM-bound steps. No tag starts a workflow, and `release.yml`
runs only by hand (`workflow_dispatch`).

**`ci.yml`** runs only hermetic steps: `gate-macos` the hermetic gate (`G4_GATE_HERMETIC=1 bash scripts/gate.sh`),
`gate-windows` only `cargo clippy` and `cargo test --workspace --all-features --no-fail-fast` (`target/` and `TEMP` on a
Dev Drive). `gate-macos` runs on every pull request against `main`, on every push to `main` and on every manual start,
`gate-windows` only with the input `windows` (so only by hand):

```sh
gh workflow run ci.yml                    # gate-macos only
gh workflow run ci.yml -f windows=true    # plus gate-windows (for example after a change that affects Windows)
gh workflow run ci.yml --ref <branch>     # a branch instead of main (it must be pushed)
```

**Release, step 1: `scripts/release-local.sh <tag> [--dry-run]`** on the Mac (macOS arm64), from the repository root:

1. Checks before the build: `version.sh check <tag>`, clean working tree, HEAD equal to `main` equal to `origin/main` (after
   `git fetch origin main`), `check-no-rom-data.sh` over the repository, the release notes (the section of the version without
   suffix from `CHANGELOG.md`, `scripts/release/changelog-section.sh`). Without `--dry-run`, additionally: `gh` is signed in,
   there is no release for the tag yet (not even a draft), and an existing tag (local or on `origin`) points to HEAD.
2. Build as the job `bundle-macos` once did: `pnpm install --frozen-lockfile`, then `CI=true pnpm bundle -- --bundles app,dmg`
   (`prepare-bundle.sh` builds apicula and rotom-lsp according to `scripts/tool-versions.env` if they are missing in `tools/`; `cargo-about`
   in the version `CARGO_ABOUT_VERSION` must be installed), then `scripts/release/macos-dmg.sh` with the hdiutil fallback.
3. Check: `check-bundle.sh`, `check-no-rom-data.sh <app> <dmg>` and the smoke test (`smoke-packaged.sh`, empty environment) on
   a synthetic project from `make_synthetic`.
4. `dist-release/` (git-ignored) with `Gen4-Studio_<version>_aarch64.dmg`, `SHA256SUMS.txt` (`shasum -a 256`) and `notes.md`.
5. Without `--dry-run`: `git tag -a <tag>` (if it is missing), `git push origin <tag>` and
   `gh release create <tag> --draft [--prerelease] --title "Gen4 Studio <version>" --notes-file dist-release/notes.md` with the DMG
   and `SHA256SUMS.txt`. A tag with `-rc` is created as a pre-release by the script.

With `--dry-run` everything runs except tag, push and release; the last line is `release-local: Probelauf ok (<files>)` (the script's messages are German).
`G4_RELEASE_ALLOW_BRANCH=1` allows a dry run outside `main` (for example in a worktree), only with `--dry-run`;
`G4_RELEASE_SKIP_BUILD=1` is only for `scripts/tests/release-local.test.sh`. The dry run of 5 November 2026 (`v0.1.0-rc.2`)
took about 5 minutes after a release build of a good 3 minutes, so cold about 10 minutes.

**Release, step 2: `release.yml`** (Windows) builds the installer on `windows-latest` from the tag and checks it
(`check-bundle-windows.sh`, `check-no-rom-data.sh`, smoke test):

```sh
gh workflow run release.yml -f tag=vX.Y.Z -f dry_run=false   # attach the installer to the draft
gh workflow run release.yml -f tag=vX.Y.Z                    # dry run (dry_run=true): build and check, upload nothing
```

The job `version` checks the version of the tag against the tag, `bundle-windows` builds and checks, and `publish-windows` (only without
`dry_run`) uploads `Gen4-Studio_<version>_x64-setup.exe` to the **existing** draft, adds the line of the installer to its `SHA256SUMS.txt`
(`scripts/release/add-checksum.sh`; a line of the same name is replaced) and uploads it again with `--clobber`.
If there is no release for the tag or it is no longer a draft, the job aborts with a message. A cold run costs
about $0.70 (Windows minutes count double); caches belong to the ref the workflow starts from (default `main`), so a
later run reads what the previous one saved.

After that the maintainer checks the draft (DMG, installer, `SHA256SUMS.txt` with both lines, text) and publishes it by hand.

**Versions:** The version lives in one place, `version` in `[workspace.package]` of `Cargo.toml`; `apps/studio/package.json`
carries the same number (Tauri reads it from there). `scripts/version.sh` prints it (`version.sh`), sets it (`version.sh set <version>`,
also writes `Cargo.lock`) and checks it (`version.sh check [<tag>]`: both places equal, valid SemVer, a section
in `CHANGELOG.md`, the tag is `v<version>`). Tags are named `vX.Y.Z` and `vX.Y.Z-rc.N`; for `-rc` the section of the version without suffix applies.
Procedure: `version.sh set`, add a section to `CHANGELOG.md`, `scripts/gate.sh`, merge and push `main`, then
`bash scripts/release-local.sh vX.Y.Z` (tag, push, draft with the DMG), `gh workflow run release.yml -f tag=vX.Y.Z -f dry_run=false`
(adds the installer), check the draft and publish it.

## Licence notices

Every bundle contains the folder `licenses/` (macOS `Contents/Resources/licenses/`, Windows `<installation folder>\licenses\`):

- `THIRD-PARTY-NOTICES.txt`: generated by `apps/studio/scripts/third-party-notices.sh` (called by `prepare-bundle.sh`).
  `cargo-about` supplies the Rust crates (template `about.hbs`, allowlist `about.toml`, which matches the list in `deny.toml`
  exactly), followed by the production packages of `apps/studio` from `pnpm licenses`.
- the GPL and LGPL texts of nitroarc and `nitroarc-AUSTAUSCH.txt` (the instructions below, in the package);
- the licences of apicula (0BSD) and rotom (MIT).

`cargo-about` is a build tool (licence MIT OR Apache-2.0), not a dependency of the product. Its version is
`CARGO_ABOUT_VERSION` in `scripts/tool-versions.env`.

The licence check stays strict: `cargo deny` (`deny.toml`) and `scripts/check-licenses-npm.sh` allow only the allowlist.
The same holds for the GitHub Actions: `.github/actions-allowlist.txt` names every permitted action with its licence,
and `scripts/check-actions.sh` checks that every `uses:` line is pinned to a 40-character SHA and is on the list.
That is why there is no `Swatinem/rust-cache` (LGPL-3.0); the cache runs through `actions/cache` (MIT).

## Replacing nitroarc

nitroarc is under the LGPL and therefore stays a separate file. If you want to use your own version:

- macOS: replace `Contents/Frameworks/libnitroarc_ffi.dylib` in the package, then sign the dylib and the app ad hoc again
  (otherwise macOS refuses to start it):

  ```sh
  codesign --force --sign - "/Applications/Gen4 Studio.app/Contents/Frameworks/libnitroarc_ffi.dylib"
  codesign --force --sign - "/Applications/Gen4 Studio.app"
  ```

- Windows: replace `nitroarc_ffi.dll` next to `studio.exe` in the installation folder. There is no signature, so
  nothing else is needed.

Your own library must have the same name and offer the same interface as the bundled one. `licenses/nitroarc-AUSTAUSCH.txt` in the package says the same.

## Checking the packaged program

- `studio doctor [--json]` reports whether the sidecars are found and nitroarc loads; with `--json` for scripts
  (`"nitroarc": true`). On Windows, `studio.exe` is a GUI program: started from `cmd` or PowerShell, `doctor` attaches
  to their console and writes there, but the shell does not wait for it (the output arrives after the
  prompt, `%ERRORLEVEL%` stays old). To wait and get the exit code: `studio.exe doctor | more` (cmd) or
  `studio.exe doctor | Out-Host` (PowerShell); `check-bundle-windows.sh` reads the output through a pipe.
- `scripts/smoke-packaged.sh <studio binary> <project>` calls `studio doctor` in an empty environment (`env -i`, no `G4_*`) and
  then `studio mcp` over stdio on a synthetic project (change a collision, read, save). The project comes from
  code, without a ROM: `cargo run -p g4-kernel --example make_synthetic --features test-support -- <folder>`.
- `scripts/check-no-rom-data.sh [--history <range>] [path …]` looks for ROM data in files, folders, DMG and history; also in
  every release run over the finished packages.

## Check commands (macOS)

`scripts/check-bundle.sh` checks:

- `Contents/MacOS/apicula` is executable and starts (`--help`).
- `Contents/MacOS/rotom-lsp` is executable, has the rpath `@executable_path/../Frameworks` and starts without a dyld error.
- `Contents/Frameworks/libnitroarc_ffi.dylib` exists and has the install name `@rpath/libnitroarc_ffi.dylib`.
- The Studio binary has the rpath `@executable_path/../Frameworks` and loads the dylib through `@rpath/...`;
  no absolute path into `target/` is linked.
- The icon of the bundle (`Contents/Resources/<CFBundleIconFile>` from `Info.plist`, `.icns` appended if it is missing) exists
  and has the same SHA-256 as `apps/studio/src-tauri/icons/icon.icns`, and that is not the hash of the old
  default icon of create-tauri-app (constant `OLD_DEFAULT_ICNS` in the script).

In addition, by hand:

```sh
APP="target/release/bundle/macos/Gen4 Studio.app"
otool -L "$APP/Contents/MacOS/studio"      # line with @rpath/libnitroarc_ffi.dylib
env -u G4_APICULA "$APP/Contents/MacOS/studio"   # starts without a dyld error
DYLD_PRINT_LIBRARIES=1 "$APP/Contents/MacOS/studio" 2>&1 | grep nitroarc   # path is in Contents/Frameworks
```

Acceptance by hand: start the `.app` from Finder, open a project (a copy) and load a header with models
(`G4_APICULA` not set); the models then come from the bundled apicula.

## App icon

The icon of the app is the brand mark from the design system (`design-system/README.md`, section "Marke"). The sources are
`design-system/marke/gen4-studio-marke.svg` and `design-system/marke/gen4-studio-marke-1024.png`; `design-system/scripts/marke.py`
writes both from the tokens. All sizes under `apps/studio/src-tauri/icons/` are made from them with
`tauri icon`:

```sh
python3 design-system/scripts/marke.py                         # SVG and 1024 px PNG anew, from the repository root
cd apps/studio
pnpm tauri icon ../../design-system/marke/gen4-studio-marke-1024.png
```

`tauri icon` also creates `64x64.png`, `android/` and `ios/`. They are not checked in (deleted) because
`tauri.conf.json` does not name them; the Windows Store logos (`Square*Logo.png`, `StoreLogo.png`) stay checked in
as before. Under `tauri dev` the Dock shows `icons/icon.png`; in the bundle `icons/icon.icns` goes to
`Contents/Resources/` (name in `CFBundleIconFile`); the About dialog uses the bundle icon.

## Signing and notarizing (prepared, off)

`scripts/release-local.sh` contains the function `sign_notarize`, which only reports that signing is not enabled; the header of the
script describes the steps (formerly the step `sign-notarize` with `if: false` in `release.yml`). With a Developer ID
it would be enabled like this:

1. Join the Apple Developer Program (paid) and create a "Developer ID Application" certificate; it then sits in the
   login keychain of the Mac (a Base64 `.p12` in `APPLE_CERTIFICATE` with `APPLE_CERTIFICATE_PASSWORD` is needed only where
   the certificate is in no keychain).
2. Set `signingIdentity` in `apps/studio/src-tauri/tauri.macos.conf.json` to the name of the identity (for example "Developer ID
   Application: Name (TEAMID)", alternatively export `APPLE_SIGNING_IDENTITY` before the bundle step) and enable Hardened Runtime
   again (section "Signature" above).
3. Notarization: Apple ID (`APPLE_ID`), an app-specific password (`APPLE_PASSWORD`, not the sign-in password; alternatively
   an API key) and the team ID (`APPLE_TEAM_ID`) in the environment of the bundle step: the Tauri bundler signs the `.app`
   and calls `notarytool` as soon as the variables are set.
4. In `sign_notarize`, insert the lines from the header of the script: notarize the DMG (`xcrun notarytool submit … --wait`) and
   staple the ticket (`xcrun stapler staple`).

On Windows a code-signing certificate would be needed; without one the SmartScreen warning stays. There is no auto-update yet.

## Known limits

- Neither package is signed with a Developer ID or notarized (macOS ad hoc, Windows not at all); the ways to do it
  are described above.
- Only macOS arm64 and Windows x64; Linux is not a target of this phase.
- Auto-update and signing with a Developer ID follow later.
