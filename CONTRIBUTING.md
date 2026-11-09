# Contributing to Gen4 Studio

Thank you for wanting to help. Gen4 Studio is an editor for Pokémon Gen 4 (Diamond, Pearl, Platinum, HeartGold, SoulSilver) ROM hacks, MIT licensed. Bug reports, ideas, documentation and code are all welcome.

By taking part you agree to the [Code of Conduct](CODE_OF_CONDUCT.md). Security problems do not belong in a public issue, see [SECURITY.md](SECURITY.md).

## Ways to contribute

- **Questions and ideas:** start a thread in [Discussions](https://github.com/Kaago/gen4-studio/discussions).
- **Bugs:** open an issue with the bug form. Say which version, platform and game you use, and give the steps to reproduce.
- **Features:** open an issue with the feature form before you write a larger change, so that we can agree on the shape first.
- **Pull requests:** small and focused ones are reviewed fastest. One concern per pull request.

## No game data, ever

Do not attach a ROM, a save game, or any file derived from one (extracted game files, patches, screenshots of extracted assets) to an issue, a pull request or a test. Gen4 Studio contains no ROM data, and `scripts/check-no-rom-data.sh` fails the gate when a tracked file looks like one. Tests run on synthetic projects. If a bug only shows with a real ROM, describe it, name the game and region, and let the maintainer reproduce it.

## Set up

The setup is described in [docs/dev-setup.md](docs/dev-setup.md) (currently in German). In short, you need:

- Rust: the toolchain of `rust-toolchain.toml` (install it with `rustup toolchain install`).
- Node and pnpm for the app in `apps/studio` (`cd apps/studio && pnpm install`).
- `cargo-deny`, `uv` and a bash for the gate.

The pinned versions of the external tools (Node, pnpm, cargo-deny, apicula, rotom, cargo-about) live in one file, `scripts/tool-versions.env`.

## Workflow

1. Fork the repository and branch from `main`.
2. Make your change, with tests. A bug fix starts with a test that fails.
3. Run the hermetic gate (next section) until it is green.
4. Open a pull request against `main` and fill in the template.

## The gate

Run this before you open a pull request:

```sh
G4_GATE_HERMETIC=1 bash scripts/gate.sh
```

The hermetic gate needs no ROM, no save game and no other tool: formatting, clippy, licenses and advisories of the dependencies, the shell script tests, the GitHub Actions audit, the ROM data guard, the version check, the Rust tests, the frontend tests and build, the design checks (tokens, styles, depth) and the unit tests of the emulator scripts. GitHub Actions runs the same on every pull request (job `gate-macos`), and a pull request is merged only when it is green.

The second part of the full gate runs the tests that need the maintainer's test ROMs and projects. It runs on the maintainer's machine before a merge; you do not need to run it.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/), in English, imperative mood, subject line short:

```
feat(world): add a warp picker to the area editor
fix: keep the selection when a map is reloaded
docs(oss): describe the release flow
```

Common types: `feat`, `fix`, `docs`, `test`, `refactor`, `build`, `ci`, `chore`. The scope is optional.

## Language

- Code comments, commit messages, issues and new documentation are in English.
- Every user-facing string goes into both `de` and `en` catalogs, on the frontend and in Rust. English is the default and the fallback; German is complete. Never add a literal user-facing string to the code.
- Codes are identifiers, not text, and are not translated: `CmdError.code`, check codes, MCP tool names, `schemaVersion`.

## Code and dependencies

- Follow the style of the code around your change. `cargo fmt`, clippy with `-D warnings` and the style checks of the gate are the reference.
- The version is set only by `bash scripts/version.sh set <version>`; do not edit it by hand and do not bump it in a pull request.
- A new dependency must pass the license checks of the gate (`cargo deny` with `deny.toml`, `scripts/check-licenses-npm.sh`). A new GitHub Action must be pinned to a commit SHA and listed in `.github/actions-allowlist.txt`.
- Gen4 Studio shares no code with DSPRE (AGPL-3.0) or any other project with an incompatible license. Do not copy code from them; format knowledge written down in your own words is fine.

## License

Your contribution is licensed under the [MIT License](LICENSE), like the rest of the project.
