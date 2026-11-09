Thank you to the DSPRE team and the whole DS hacking community: **Gen4 Studio**, a text-based, Git-friendly editor for Pokémon Platinum, HeartGold and SoulSilver ROM hacks, is out as v1.0.0, and it only exists because of your work.

- Your hack is a folder of text files (one JSON file per entity), so changes are readable diffs in Git
- Build worlds, stories, scripts and data, then build a ROM or an xdelta patch
- Built-in MCP server (90 tools, read/write permissions, local only) for AI agents like Claude Code
- Opens DSPRE project folders. Independent reimplementation in Rust and TypeScript, shares no code with DSPRE, which was our test oracle and stays the right tool for its many editors

**Download** for macOS (Apple silicon) and Windows (x64); not signed yet, so Gatekeeper/SmartScreen asks on first start (steps in the README)
https://github.com/Kaago/gen4-studio/releases/latest
README: <https://github.com/Kaago/gen4-studio#readme>
MCP guide: <https://github.com/Kaago/gen4-studio/blob/main/docs/en/mcp.md>

MIT licensed. No ROM data inside: bring your own ROM. Pokémon and Nintendo trademarks belong to their owners. Bug reports and feedback welcome as GitHub issues.
