# Changelog

All notable changes to Gen4 Studio are recorded in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the versions follow
[Semantic Versioning](https://semver.org/). The version lives in one place,
`version` in `[workspace.package]` of `Cargo.toml`; `bash scripts/version.sh set <version>`
keeps `apps/studio/package.json` and `Cargo.lock` in line. The release workflow uses the
section of the version as the text of the draft release.

## [1.0.2] - 2026-11-12

### Changed

- Scripts compile with rotom v0.8.1 (was v0.7.0), and the language server of the script editor comes from the same version. The script language can now use autovar for variable parameters and choose the result variable with `.into(...)`, and `format()` takes text over several lines.
- Updated dependencies: base64 0.23, sha1 0.11, getrandom 0.4, uxie 0.9.8, rmcp 3.9.0, tauri-plugin-dialog 2.8.4 and cc 1.11.1 on the Rust side; dagre 3.1, TypeScript 7.0, jsdom 31, CodeMirror, React Flow 12.12.1, dockview 8.9, three.js 0.187, Vite 8.3, Vitest 5.1 and the Tauri CLI 2.12.4 in the interface; taiki-e/install-action 2.90.3 in the build.

### Fixed

- "Adopt scripts" refers to common scripts by the labels the same run gives them, and no longer takes the end of a movement for the start of the next one (rotom v0.8.1).
- In DSPRE projects, the constants for items, Pokémon, places, trainers, moves and sounds come from the text archives of the project's game and language (rotom v0.8.1).

### Security

- rmcp 3.9.0 fixes RUSTSEC-2026-0462; the affected HTTP client code was never compiled into Gen4 Studio.
- The open advisories for zip 8.6, which comes in through rotom, do not reach Gen4 Studio: rotom reads only its own command database and uses none of the affected functions. They stay open until rotom moves to zip 9.

## [1.0.1] - 2026-11-12

### Added

- Auto-update from GitHub Releases. On the first start Gen4 Studio asks once whether it may check for a new version; if yes, it checks at startup, at most once a day. Settings › Updates has a switch, "Check now", the version and the last check, and the menu has "Check for Updates…". A new version shows a notice with "Install…" and "Skip this version"; installing downloads the update, warns about unsaved changes, installs it and restarts the app (on Windows the app closes while the installer runs). Updates are signed with the project's own key, and nothing about your projects is sent. If you run 1.0.0, install 1.0.1 once by hand.

### Fixed

- Switching the language tells the backend first and reloads the texts it built (overview, data, checks, problems, project info, Git status).
- Errors carry codes instead of being recognized by their translated text.
- The remaining English refusals and errors in the German interface are translated.
- Palette groups have language-neutral ids.
- The prefilled reward text follows the language of the ROM, not of the interface.
- Flag fields read correctly inside a sentence.
- Cleaned up duplicate entries in the message catalog.

## [1.0.0] - 2026-11-10

### Added

- Phase 1: kernel foundation and shell spike: open a project, paint permissions, save, build a ROM and run it in the emulator (HeartGold, SoulSilver, Platinum).
- Phase 2 and 3a: world editor with header, matrix, buildings, events, warps and 3D models; World explorer, wizards for new areas and rooms, World map.
- Phase 3b: new headers by patch, scripts with rotom and CodeMirror 6, text editor, data editors (Pokédex, moves, items, trainers, encounters), Check and fix, import of finished map parts.
- Phase 3c and 3e: the design system in the app with tokens and a component library, a frame of title bar, atlas, list and View; the Workbench with a world tree and a dock per area.
- Phase 3f and 3g: world model with own places as place names in the ROM; "New game from ROM…" with templates.
- Phase 3h: agent interface (MCP): curated tools through `studio mcp`, headless and in the running app, permission per project.
- Phase 4: the text project as the source, a build graph for "Play here", "Reset to original", minimal Git in the project.
- Phase 4.5: "Publish…": a wizard in four steps (version, patch, check, done) writes a self-checked xdelta patch `<project>-v<version>.xdelta` with a sidecar file (version, date, change notes, base and checksums) and optionally a `README.txt`, and records the release in `.g4/releases.json`; "Save a snapshot" commits the project to its own local Git repository (no remote, no push). The MCP tools `publish_patch` and `git_commit_all` and the commands `g4k publish` and `g4k patch create` do the same headless.
- Phase 3d: Tell: flags and variables, characters, dialogs and doors, trainers by wizard, deleting with a reference lock, battle sprites and egg moves, reading and telling stories with steps and recipes, and a live play mode that reads and writes the save state of the running game in melonDS.
- Phase 6: Map creation and world format: paint terrain from tiles, bake the NSBMD model, the BDHC terrain and the permissions from it, and play the map; the world format in the text project (layers, heights, ramps, tilesets per texture pack, hand-set permissions), Smart Drawing with edge sets and cliffs, import of PNG and 3D models (OBJ, glTF, GLB), freeform models and exchange with Tiled (TMJ).
- English and German interface: the app, the Rust-side messages, the `g4k` command line and the MCP answers follow the system language or the choice in Settings (System, Deutsch, English). English is the default and the fallback; German is complete. Headless programs read `G4_LANG`.
- Open source: MIT license, README in English and German, contributing guide, code of conduct, security policy, issue forms and a pull request template; English guides for MCP, development setup and release in `docs/en/`; CI runs the hermetic gate on every pull request.
- Release: one script (`scripts/version.sh`) keeps the version equal for Cargo, npm and Tauri.
