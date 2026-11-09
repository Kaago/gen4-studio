# Gen4 Studio 1.0: a text-based, Git-friendly editor for Platinum, HeartGold and SoulSilver ROM hacks

## What it is

Gen4 Studio is a cross-platform editor for Pokémon Platinum, HeartGold and SoulSilver ROM hacks. You open your own ROM once, and from then on the hack is a folder of text files that you edit in the app, by hand or with an AI agent, keep in Git, and build into a ROM or an xdelta patch. It runs on macOS and Windows, speaks English and German, and is MIT licensed.

## What you can do

- **Build the world.** Explore places in 2D and 3D, create areas and rooms with wizards, edit events and warps, and paint maps from tiles.
- **Tell stories.** Lay out steps tied to flags and variables; place characters, doors and trainers.
- **Scripts, text and data.** Edit rotom scripts, game text with a DS screen preview, and Pokédex entries, moves, items, trainers and encounters. Dozens of project checks catch broken references and too-wide text.
- **Text project with Git.** Game data is JSON, one entity per file, so changes are readable diffs.
- **Build and share.** An unchanged project builds a ROM identical to the original. "Publish…" writes a self-checked xdelta patch, so you share the patch, never the game.

## Download and install

Get it from the [release page](https://github.com/Kaago/gen4-studio/releases/latest): a DMG for macOS 13+ (Apple silicon) and an installer for Windows 10+ (x64). The packages are not signed yet, so your system warns you on the first start. On macOS, start the app once, then choose "Open Anyway" in System Settings, Privacy & Security. On Windows, when SmartScreen shows "Windows protected your PC", choose "More info", then "Run anyway". See the [README](https://github.com/Kaago/gen4-studio#readme) for details. You need your own `.nds` ROM.

## Agents (MCP)

A built-in MCP server lets an AI agent such as Claude Code read and edit the open project: 90 curated tools, steps in the undo history, everything local. Access is off by default and granted per project. See the [MCP guide](https://github.com/Kaago/gen4-studio/blob/main/docs/en/mcp.md).

## Relationship to DSPRE

Gen4 Studio would not exist without DSPRE, the DS Pokémon ROM Editor, and the reverse-engineering work of its maintainers and the DS hacking community. DSPRE is our reference for Gen IV format knowledge and our test oracle: Gen4 Studio produces byte-identical output on unchanged data. It is an independent reimplementation in Rust and TypeScript and shares no code with DSPRE. It reads and writes the DSPRE project layout, so you can open an existing DSPRE project and use both tools side by side. For the full breadth of DSPRE's editors, DSPRE remains the right tool.

## What is missing

- Verified: Platinum (German) and HeartGold (German and US). SoulSilver and other regions are untried; Diamond and Pearl are recognized but not enabled.
- Windows is built and smoke-tested in CI, but not yet checked by hand. The live play mode is macOS only.
- Planned: a Linux build, signed and notarized packages, English format docs. Bugs and ideas are welcome on [GitHub](https://github.com/Kaago/gen4-studio/issues); never attach a ROM or save.

## Legal

Gen4 Studio contains no ROM data and no game assets; you need your own legally obtained copy of the game. Pokémon and all related names are trademarks of their respective owners, and this project is not affiliated with or endorsed by them.

## Thanks

Thank you to the DSPRE team and the whole DS hacking community, including the DS Pokémon Hacking Wiki, the pret decompilations and the Kingdom of DS ROM Hacking community, for the shared knowledge that made this possible.
