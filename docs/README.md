# Gen4 Studio documentation

This is the map of the `docs/` folder. If you are new here, start with the project `README.md` in
the repository root (`README.de.md` is the German version). The three guides on MCP, setup and
releases exist in English (`docs/en/`) and in German (the files next to this one); the format notes
and the development record are written in German (details below).

## For users and agents

- **MCP interface for coding agents.** [mcp.md](mcp.md) describes how an agent such as Claude Code
  connects to Gen4 Studio (inside the running app or headless via `studio mcp`), the curated tools
  with their English names and schemas, resources, example prompts, write confirmation and
  per-project consent. The English version lives in `docs/en/mcp.md`.
- **Format notes.** [formats/](formats) has one file per game format or data topic: NARC, maps,
  matrices, NSBMD/NSBTX, text archives, scripts, personal data, learnsets, trainers, encounters, the
  text project, the project base mark and more. They record what Gen4 Studio knows about the Gen 4
  file formats and where the code for each lives. They are written in German, each opens with a
  short English summary, and a full English translation is planned after 1.0.
  [formats/text_project.md](formats/text_project.md) describes the text-based project (JSON under
  `data/`, made for Git); [formats/project_base.md](formats/project_base.md) names the base ROM of a
  project.
- **Releases.** [../CHANGELOG.md](../CHANGELOG.md) lists what changed in each version. How a
  release is built is under "For contributors".

## For contributors

- **Setup.** [dev-setup.md](dev-setup.md) covers the external tools (apicula, rotom and rotom-lsp),
  how to run the tests, the gated tests that need a test project, the design system and tokens, the
  agent test environment and the app bundle. The English version lives in `docs/en/dev-setup.md`.
  `CONTRIBUTING.md` in the repository root explains the workflow for pull requests.
- **The gate.** [../scripts/gate.sh](../scripts/gate.sh) is the local gate: formatting, lints,
  dependency licenses, the shell script tests ([../scripts/tests/run.sh](../scripts/tests/run.sh)),
  the ROM data guard ([../scripts/check-no-rom-data.sh](../scripts/check-no-rom-data.sh)), the link
  check ([../scripts/check-links.sh](../scripts/check-links.sh)), the full test suite and the
  frontend. With `G4_GATE_HERMETIC=1` it runs only the part that needs no ROM, tool or save game,
  which is what CI runs and what every pull request has to pass. [agent-gate.md](agent-gate.md)
  tells an agent how to run the gates that need the real UI and the emulator.
- **Release.** [release.md](release.md) describes the macOS DMG and the Windows installer, tags,
  versions, license notices and the checks of the packaged app; the English version lives in
  `docs/en/release.md`. [briefing-release-ci.md](briefing-release-ci.md) and
  [release-ci-gate.md](release-ci-gate.md) record how release and CI were set up.
- **Design system.** [../design-system/README.md](../design-system/README.md) describes the
  "Doppeldeck" design system: tokens, the OpenPencil file, the briefings for drawing screens.

## Development history

These documents are the working record of how Gen4 Studio was built. They are kept as they were
written, in German, and are not maintained as user documentation. They may mention tools, folders
and test projects on the maintainer's machine; no ROM data is part of the repository.

- [ROADMAP.md](ROADMAP.md): the phases, what each delivers, its gate and its state.
- [superpowers/specs/](superpowers/specs): one design specification per phase, starting with
  [the overall design](superpowers/specs/2026-10-21-gen4-studio-design.md).
- [superpowers/plans/](superpowers/plans): the implementation plans, task by task.
- `phase-*-gate.md`, for example [phase-1-gate.md](phase-1-gate.md),
  [phase-3a-gate.md](phase-3a-gate.md), [phase-4-4-gate.md](phase-4-4-gate.md) and
  [phase-6-5b-gate.md](phase-6-5b-gate.md): the gate checklists with their results.
- [research/](research): analyses, expert reports, proposals and decisions that came before the
  specs. The first one,
  [the analysis of DSPRE and the tool landscape](research/2026-10-20-dspre-und-suite-analyse.md),
  is an internal architecture assessment from October 2026 that served as the basis for the
  decision to build Gen4 Studio. It is not a critique of DSPRE, which has had further releases
  since then.

### How it was built

Gen4 Studio was built in numbered phases (1 to 6, split into parts such as 3b.3 or 6.4a). Each
phase starts with a research round where needed, a specification and a plan, and ends with a gate:
a checklist that combines the automated tests of `scripts/gate.sh` with checks an agent runs against
the real UI and an emulator, and items marked for a human look ("Blick") or hands-on test ("Hand")
that the maintainer completes. The implementation was agentic: AI coding agents worked through the
plans task by task, each phase was then accepted by an independent review, and the findings were
fixed before the gate counted as passed. The roadmap records, phase by phase, what passed and what
is still open with the maintainer.
