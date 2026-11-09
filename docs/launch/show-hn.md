# Show HN post

Submit with the repository as the URL and leave the text field empty; then post the author comment below as the first comment, right after submitting.

## Title to submit

Hacker News limits titles to 80 characters. This one has 77:

```
Show HN: Gen4 Studio – Git-friendly editor for Pokémon DS ROM hacks, with MCP
```

The long form of the title below has 103 characters and exceeds the limit, so the submit form rejects or truncates it. It is the full wording of the pitch; use it without the "Show HN: " prefix for places without that limit, such as the headline of a forum thread:

```
Show HN: Gen4 Studio – a text-based, Git-friendly editor for Pokémon DS ROM hacks with an MCP interface
```

## URL

```
https://github.com/Kaago/gen4-studio
```

## Author comment

```
Gen4 Studio is a desktop editor (macOS and Windows) for Pokémon Platinum, HeartGold and SoulSilver ROM hacks whose project is a folder of text files, so a hack can live in Git, be reviewed as diffs and be driven by AI agents over MCP. Thank you to the DSPRE team and the DS hacking community: none of this exists without their reverse-engineering work.

Motivation: ROM hacks are usually binaries touched by GUI tools, which are hard to diff or review. I wanted to treat a hack like a software project. You open your own ROM once; it is unpacked into a DSPRE-style project folder plus a text layer, with game data as canonical JSON (one entity per file, stable IDs). Build produces a ROM or an xdelta patch, and an unchanged project builds a ROM identical to the original, so you can share a patch and never the game.

Architecture: a Rust workspace is the kernel. Every edit is a command with undo, and the UI and the MCP server go through the same commands. The app is Tauri 2 with React 19, TypeScript and three.js for the 2D and 3D views. The MCP server (`studio mcp`) offers 90 curated tools, headless over stdio, or in the running app over local HTTP (127.0.0.1); access is off by default and granted per project. Guide: https://github.com/Kaago/gen4-studio/blob/main/docs/en/mcp.md

On DSPRE: the formats are re-implemented from the community's documentation and decompilation work, with DSPRE as the test oracle (byte-identical output on unchanged data). No code is shared (DSPRE is AGPL-3.0, Gen4 Studio is MIT), but it reads and writes DSPRE's project layout. DSPRE has many more editors and stays the right tool for those.

Limits:
- You cannot try it without a ROM: it needs your own legally obtained .nds of one of the three games, and the repository contains no ROM data. Without one you can read the README and the MCP guide or run the hermetic tests.
- Verified: Platinum (German) and HeartGold (German and US). SoulSilver and other regions are untried; Diamond and Pearl are not enabled.
- Windows is built and smoke-tested in CI but not checked by hand on a PC. Live play mode is macOS only, there is no Linux build, and the packages are unsigned.

Download: https://github.com/Kaago/gen4-studio/releases/latest
README: https://github.com/Kaago/gen4-studio#readme

Feedback wanted: does one JSON file per entity hold up in Git for a big hack? Pokémon and Nintendo trademarks belong to their owners; no affiliation.
```
