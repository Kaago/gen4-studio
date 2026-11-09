# Reddit post for r/PokemonROMhacks

Post as a text post (or a gallery post with the first paragraph as the first comment, depending on what the subreddit currently allows). Before posting: read the subreddit's sidebar rules, pick the flair that fits a tool or resource release, and replace the screenshots line below with the link to the uploaded images (the three images in `docs/media/`, nothing with game graphics beyond what the app itself shows). Stay online for the first hours and answer comments.

## Title

Gen4 Studio 1.0: a new editor for Platinum, HeartGold and SoulSilver hacks with a text-based project, Git, xdelta patches and MCP for AI agents

## Body

First of all, a big thank you to the DSPRE team and to everyone in the DS hacking community who documented these games. Gen4 Studio would not exist without that work.

**What it is.** Gen4 Studio is a cross-platform editor (macOS and Windows) for Pokémon Platinum, HeartGold and SoulSilver ROM hacks. You open your own ROM once, and from then on your hack is a folder of text files: maps, scripts, texts and game data, one JSON file per entity, so every change is a readable diff in Git. You build worlds (2D and 3D, with tile-based map painting), tell stories (characters, dialogs, doors, trainers, flags and variables), edit scripts in the rotom language and edit data (Pokédex, moves, learnsets, items, trainers, encounters). Dozens of project checks catch broken references and text that is too wide for the DS screen before the game does. When you are done, it builds a ROM or an xdelta patch, so you can share your work without sharing the game.

**Works with DSPRE.** Gen4 Studio reads and writes the DSPRE project layout, so you can open an existing DSPRE project and keep using both tools side by side. It is an independent reimplementation in Rust and TypeScript and shares no code with DSPRE; DSPRE was our reference for the formats and our test oracle (an unchanged project builds a ROM identical to the one you started from). If you need the full breadth of DSPRE's editors, DSPRE is still the right tool.

**AI agents.** A built-in MCP server lets agents such as Claude Code read and edit the project you have open: 90 curated tools, access is off by default, you grant it per project (read, or read and edit), and everything stays on your machine. This is optional; the app works fine without it.

**Screenshots:** [add the link to the uploaded screenshots here before posting]

**Download:** https://github.com/Kaago/gen4-studio/releases/latest (macOS Apple silicon DMG, Windows x64 installer). The packages are not signed with a developer certificate yet, so macOS Gatekeeper ("Open Anyway") and Windows SmartScreen ("Run anyway") ask on the first start; the README explains the steps.
**Source and docs:** https://github.com/Kaago/gen4-studio#readme (MIT license). The MCP guide is at https://github.com/Kaago/gen4-studio/blob/main/docs/en/mcp.md.

**What is missing, honestly:**

- Platinum is verified with a German ROM and HeartGold with German and US ROMs; other regions run through the same code but are untried, and SoulSilver has no test project yet.
- Diamond and Pearl are recognized but not enabled.
- Windows is built and smoke-tested in CI, but not yet checked by hand on a real PC. The live play mode (reads and sets flags and variables in the running game) needs melonDS 1.1 or later and works on macOS only.
- No Linux build, and no signed or notarized packages yet.
- It covers world building, storytelling, scripts, text and the main data editors, not all of what DSPRE can edit.

**Legal.** Gen4 Studio contains no ROM data and no game assets. You need your own legally obtained ROM; please do not ask for or post ROMs here. Pokémon and all related names are trademarks of their respective owners, and this project is not affiliated with or endorsed by them.

**Feedback.** I would love to hear what blocks you from trying it on your own hack, what you would want next, and what breaks. GitHub issues are the best place for bugs (never attach a ROM, a save or any file derived from one), and comments here are welcome too.
