# Show HN post

Submit with the repository as the URL and leave the text field empty; then post the author comment below as the first comment, right after submitting.

## Title

```
Show HN: Gen4 Studio – a text-based, Git-friendly editor for Pokémon DS ROM hacks with an MCP interface
```

Hacker News limits titles to 80 characters, and this title has 103. If the submit form rejects or truncates it, use this shorter one (77 characters) and keep the rest of the post unchanged:

```
Show HN: Gen4 Studio – Git-friendly editor for Pokémon DS ROM hacks, with MCP
```

## URL

```
https://github.com/Kaago/gen4-studio
```

## Author comment

```
Gen4 Studio is a desktop editor (macOS and Windows) for Pokémon Platinum, HeartGold and SoulSilver ROM hacks whose project is a folder of text files, so a hack can live in Git, be reviewed as diffs and be driven by AI agents over MCP. Thank you first to the DSPRE team and the DS hacking community: none of this exists without their reverse-engineering work.

Motivation: ROM hacks are usually a pile of binaries touched by GUI tools, which makes them hard to diff, review, merge or script. I wanted to treat a hack like a software project, and to work with places, characters, dialogs and doors instead of tables. You open your own ROM once; it is unpacked into a DSPRE-style project folder plus a text layer: game data as canonical JSON under data/, one entity per file with stable IDs, scripts in the rotom language, texts in expanded/. Build produces a ROM or an xdelta patch, and an unchanged project builds a ROM identical to the one you started from. "Publish" writes a self-checked patch, so you share the patch and never the game.

Architecture: a Rust workspace is the kernel (formats, project model, build, checks, xdelta). Every edit is a command with undo, and the UI, the g4k command line and the MCP server all go through the same commands. The app is Tauri 2 with a React 19 and TypeScript front end, and the 2D and 3D world views use three.js. The MCP server (`studio mcp`) offers 90 curated tools with English names and schemas. Headless it runs over stdio and opens a project itself; in the running app it connects over HTTP on 127.0.0.1 only, access is off by default and granted per project (none, read, read and edit), and the agent's steps show up in the undo history like yours. Guide: https://github.com/Kaago/gen4-studio/blob/main/docs/en/mcp.md

On DSPRE: the formats are re-implemented in Rust and TypeScript from the community's documentation and decompilation work, next to a few community libraries (ds-rom, uxie, rotom, chatot, apicula). DSPRE served as the test oracle: same input, byte-identical output on unchanged data. Gen4 Studio shares no code with it (DSPRE is AGPL-3.0, Gen4 Studio is MIT) and reads and writes the DSPRE project layout, so both tools can open the same project. DSPRE has many more editors and remains the right tool for those.

What is missing, and the limits:
- You cannot try it without a ROM, I am afraid. It needs your own legally obtained .nds of one of the three games, the repository contains no ROM data, and I cannot provide one. Without a ROM you can read the README with screenshots and the MCP guide, and run the hermetic tests (the part CI runs) from source.
- Verified: Platinum (German) and HeartGold (German and US). Other regions run through the same code but are untried; SoulSilver has no test project yet. Diamond and Pearl are recognized but not enabled.
- Windows is built and smoke-tested in CI but has not been checked by hand on a real PC. The live play mode (flags and variables of the running game) needs melonDS 1.1+ and works on macOS only. No Linux build.
- The packages are unsigned (macOS "Open Anyway", Windows "Run anyway"). The format docs are in German with an English summary at the top.

Download: https://github.com/Kaago/gen4-studio/releases/latest
README: https://github.com/Kaago/gen4-studio#readme

I would most like feedback on the text project layout (does one JSON file per entity hold up in Git for a big hack?) and on the MCP tool design. Pokémon and Nintendo trademarks belong to their owners; this project is not affiliated with them.
```
